# PNA Rust Project 3: Synchronous client-server networking

## Part 1: Command line parsing

这一步需要对命令进行转换，基本思路和 project 2 一样，不过这里多加了一个 `[--addr IP-PORT]`。

来看下 `kvs-client` 的要求：

The `kvs-client` executable supports the following command line arguments:

- `kvs-client set <KEY> <VALUE> [--addr IP-PORT]`

  Set the value of a string key to a string.

  `--addr` accepts an IP address, either v4 or v6, and a port number, with the format `IP:PORT`. If `--addr` is not specified then connect on `127.0.0.1:4000`.

  Print an error and return a non-zero exit code on server error, or if `IP-PORT` does not parse as an address.

- `kvs-client get <KEY> [--addr IP-PORT]`

  Get the string value of a given string key.

  `--addr` accepts an IP address, either v4 or v6, and a port number, with the format `IP:PORT`. If `--addr` is not specified then connect on `127.0.0.1:4000`.

  Print an error and return a non-zero exit code on server error, or if `IP-PORT` does not parse as an address.

- `kvs-client rm <KEY> [--addr IP-PORT]`

  Remove a given string key.

  `--addr` accepts an IP address, either v4 or v6, and a port number, with the format `IP:PORT`. If `--addr` is not specified then connect on `127.0.0.1:4000`.

  Print an error and return a non-zero exit code on server error, or if `IP-PORT` does not parse as an address. A "key not found" is also treated as an error in the "rm" command.

- `kvs-client -V`

  Print the version.

整个项目的结构：

![kvs](/rust/projects/img/kvs.png)

由于有 `IP-PORT` ，我们可以使用 [Crate `structopt`](https://docs.rs/structopt/latest/structopt/index.html#how-to-derivestructopt)。

在 `Cargo.toml` 中添加：

```toml
[dependencies]
structopt = "0.3.26"
```

先定义默认的地址和格式：

```rust
use std::net::SocketAddr;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const ADDRESS_FORMAT: &str = "IP:PORT";
```

然后在命令中使用 `structopt` 。例如 `set` 命令：

```rust
use std::net::SocketAddr;
use structopt::StructOpt;

/// A kvs client
#[derive(Debug, StructOpt)]
#[structopt(name = "kvs-client", about = "A kvs client")]
struct Cli {
    #[structopt(subcommand)]
    command: Commands,
}

#[derive(Debug, StructOpt)]
enum Commands {
    /// Set the value of a string key to a string.
    #[structopt(name = "set")]
    Set {
        /// A string key
        #[structopt(name = "key")]
        key: String,

        /// The string value of the key
        #[structopt(name = "value")]
        value: String,

        /// Sets the server address
        #[structopt(long, value_name = ADDRESS_FORMAT, default_value = DEFAULT_LISTENING_ADDRESS, parse(try_from_str))]
        addr: SocketAddr,
    },
  	// ...
}
```

用 `#[structopt(value_name)]`设置格式，`#[structopt(default_value)]` 来设置默认值。

下面实现 `kvs-server` 相关命令，看看 spec 的要求：

The `kvs-server` executable supports the following command line arguments:

- `kvs-server [--addr IP-PORT] [--engine ENGINE-NAME]`

  Start the server and begin listening for incoming connections. `--addr` accepts an IP address, either v4 or v6, and a port number, with the format `IP:PORT`. If `--addr` is not specified then listen on `127.0.0.1:4000`.

  If `--engine` is specified, then `ENGINE-NAME` must be either "kvs", in which case the built-in engine is used, or "sled", in which case sled is used. If this is the first run (there is no data previously persisted) then the default value is "kvs"; if there is previously persisted data then the default is the engine already in use. If data was previously persisted with a different engine than selected, print an error and exit with a non-zero exit code.

  Print an error and return a non-zero exit code on failure to bind a socket, if `ENGINE-NAME` is invalid, if `IP-PORT` does not parse as an address.

- `kvs-server -V`

  Print the version.

这里有两个命令选择项，`[--addr IP-PORT] [--engine ENGINE-NAME]` ，依然用 `structopt` ，不过对于 `engine` ，可以设置一个枚举类型 `Engine` ，要么是 `kvs` ，要么是 `sled` 。这里对于 `Engine` 用到了 `clap::arg_enum`。

```rust
use structopt::StructOpt;
use clap::arg_enum;

/// A kvs server
#[derive(Debug, StructOpt)]
#[structopt(name = "kvs-server", about = "A kvs server")]
struct Cli {
    //..
  
    /// Sets the storage engine
    #[structopt(
        long,
        value_name = "ENGINE-NAME",
        possible_values = &Engine::variants()
    )]
    engine: Option<Engine>,

}

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Engine {
        kvs,
        sled,
    }
}
```

## Part 2: Logging

The major difference is that `log` is fairly simple, logging only formatted strings; `slog` is feature-rich, and supports "structured logging", where log entries are typed and serialized in easily-parsed formats.

## Part 3: Client-server networking setup

服务端是单线程和同步的。

For this project, the server is synchronous and single-threaded. That means that you will listen on a socket, then accept connections, and execute and respond to commands one at a time. In the future we will re-visit this decision multiple times on our journey toward an asynchronous, multi-threaded, and high-performance database.

## Part 5: Pluggable storage engines

sled 文档: [crate sled](https://docs.rs/sled/latest/sled/)。

`sled` is a high-performance embedded database with an API that is similar to a `BTreeMap<[u8], [u8]>`, but with several additional capabilities for assisting creators of stateful systems.

It is fully thread-safe, and all operations are atomic. Multiple `Tree`s with isolated keyspaces are supported with the [`Db::open_tree`](https://docs.rs/sled/latest/sled/struct.Db.html#method.open_tree) method.

ACID transactions involving reads and writes to multiple items are supported with the [`Tree::transaction`](https://docs.rs/sled/latest/sled/struct.Tree.html#method.transaction) method. Transactions may also operate over multiple `Tree`s (see [`Tree::transaction`](https://docs.rs/sled/latest/sled/struct.Tree.html#method.transaction) docs for more info).

Users may also subscribe to updates on individual `Tree`s by using the [`Tree::watch_prefix`](https://docs.rs/sled/latest/sled/struct.Tree.html#method.watch_prefix) method, which returns a blocking `Iterator` over updates to keys that begin with the provided prefix. You may supply an empty prefix to subscribe to everything.

[Merge operators](https://github.com/spacejam/sled/wiki/merge-operators) (aka read-modify-write operators) are supported. A merge operator is a function that specifies how new data can be merged into an existing value without requiring both a read and a write. Using the [`Tree::merge`](https://docs.rs/sled/latest/sled/struct.Tree.html#method.merge) method, you may “push” data to a `Tree` value and have the provided merge operator combine it with the existing value, if there was one. They are set on a per-`Tree` basis, and essentially allow any sort of data structure to be built using merges as an atomic high-level operation.

## Part 6: Benchmarking

什么是Benchmark（参考[大数据系统 Benchmark 综述](https://yangtonghome.github.io/uploads/%E5%A4%A7%E6%95%B0%E6%8D%AE%E7%B3%BB%E7%BB%9FBenchmark%E7%BB%BC%E8%BF%B0_final.pdf)）：

> Benchmark 主要由三部分组成：数据集、工作负载和度量指标。通常 benchmark 会为使用者提供两种 程序，一种是将测试数据集装载程序，负责为被测试的计算机系统提供测试数据集，另外一种是测试的执行 程序，负责为被测试的计算机系统提供工作负载。通过这两中程序的协同配合完成对计算机系统的评测。

------

**阅读 [test](https://doc.rust-lang.org/stable/unstable-book/library-features/test.html) 笔记**

Advice on writing benchmarks:

- Move setup code outside the `iter` loop; only put the part you want to measure inside
- Make the code do "the same thing" on each iteration; do not accumulate or change state
- Make the outer function idempotent too; the benchmark runner is likely to run it many times
- Make the inner `iter` loop short and fast so benchmark runs are fast and the calibrator can adjust the run-length at fine resolution
- Make the code in the `iter` loop do something simple, to assist in pinpointing performance improvements (or regressions)

------

**[Crate criterion](https://docs.rs/criterion/latest/criterion/)**

A statistics-driven micro-benchmarking library written in Rust.

This crate is a microbenchmarking library which aims to provide strong statistical confidence in detecting and estimating the size of performance improvements and regressions, while also being easy to use.

See [the user guide](https://bheisler.github.io/criterion.rs/book/index.html) for examples as well as details on the measurement and analysis process, and the output.

[Features:](https://docs.rs/criterion/latest/criterion/#features)

- Collects detailed statistics, providing strong confidence that changes to performance are real, not measurement noise.
- Produces detailed charts, providing thorough understanding of your code’s performance behavior.

------

运行 `cargo bench` 得到的结果为：

![image-20220803190023145](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803190023145.png)

![image-20220803190043783](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803190043783.png)

可以看到应该是 sled 比较快些。

## 运行

由于有客户端和服务端，需要开启两个终端来运行。首先运行服务端：

![image-20220803183813908](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803183813908.png)

我们看到已经在进行监听了，时刻准备着服务。由于这是第一次运行，会在当前文件夹下创建 engine 文件和日志文件，目前日志中为空，等待客户端输入命令。

![image-20220803183938290](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803183938290.png)

现在客户端设置两个值，可以看到对应的日志文件中也会写入相应的命令。

![image-20220803184226113](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803184226113.png)

![image-20220803184245992](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803184245992.png)

下面来运行 sled engine。

![image-20220803184544876](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803184544876.png)

可以看到在文件目录下多了 db 、conf文件，以及 engine 中以及更改成 sled。

![image-20220803184629138](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803184629138.png)

运行结果如下：

![image-20220803184917309](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220803184917309.png)

如果服务端的端口号更改了，不是默认设置的，那么客户端也必须在该端口号下进行请求。
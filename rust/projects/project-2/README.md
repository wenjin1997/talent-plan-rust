# PNA Rust Project 2: Log-structured file I/O

> **Task**: Create a *persistent* key/value store that *can be accessed from the command line*.
>
> **Goals**:
>
> - Handle and report errors robustly
> - Use serde for serialization
> - Write data to disk as a log using standard file APIs
> - Read the state of the key/value store from disk
> - Map in-memory key-indexes to on-disk values
> - Periodically compact the log to remove stale data
>
> **Topics**: log-structured file I/O, bitcask, the `failure` crate, `Read` / `Write` traits, the `serde` crate.

## Introduction

什么是 write-ahead log ？

> In [computer science](https://en.wikipedia.org/wiki/Computer_science), **write-ahead logging** (**WAL**) is a family of techniques for providing [atomicity](https://en.wikipedia.org/wiki/Atomicity_(database_systems)) and [durability](https://en.wikipedia.org/wiki/Durability_(database_systems)) (two of the [ACID](https://en.wikipedia.org/wiki/ACID) properties) in [database systems](https://en.wikipedia.org/wiki/Database_system).[[1\]](https://en.wikipedia.org/wiki/Write-ahead_logging#cite_note-Hellerstein_Stonebraker_Hamilton_p.-1) A write ahead log is an append-only auxiliary disk-resident structure used for crash and transaction recovery. The changes are first recorded in the log, which must be written to [stable storage](https://en.wikipedia.org/wiki/Stable_storage), before the changes are written to the database.

## Part 1: Error handling

错误处理，这里用到 failure crate，[使用文档](https://github.com/pingcap/talent-plan/blob/master/courses/rust/projects/project-2/README.md)笔记如下：

### The `Fail` trait

> The `Fail` trait is a replacement for [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html). It has been designed to support a number of operations:
>
> - Because it is bound by both `Debug` and `Display`, any failure can be printed in two ways.
> - It has both a `backtrace` and a `cause` method, allowing users to get information about how the error occurred.
> - It supports wrapping failures in additional contextual information.
> - Because it is bound by `Send` and `Sync`, failures can be moved and share between threads easily.
> - Because it is bound by `'static`, the abstract `Fail` trait object can be downcast into concrete types.

#### Cause

可以得到更具体的错误类型信息，进行转换：

```rust
while let Some(cause) = fail.cause() {

    if let Some(err) = cause.downcast_ref::<io::Error>() {
        // treat io::Error specially
    } else {
        // fallback case
    }

    fail = cause;
}
```

#### Backtraces

```rust
// We don't even know the type of the cause, but we can still get its
// backtrace.
if let Some(bt) = err.cause().and_then(|cause| cause.backtrace()) {
    println!("{}", bt)
}
```

#### Context

```rust
use failure::ResultExt;

let mut file = File::open(cargo_toml_path).context("Missing Cargo.toml")?;
file.read_to_end(&buffer).context("Could not read Cargo.toml")?
```

### Deriving `Fail`

All failures need to implement `Display`.

#### Deriving `Dispaly`

```rust
extern crate failure;
#[macro_use] extern crate failure_derive;

#[derive(Fail, Debug)]
#[fail(display = "An error occurred.")]
struct MyError;
```

```rust
extern crate failure;
#[macro_use] extern crate failure_derive;

#[derive(Fail, Debug)]
#[fail(display = "An error occurred with error code {}.", _0)]
struct MyError(i32);


#[derive(Fail, Debug)]
#[fail(display = "An error occurred with error code {} ({}).", _0, _1)]
struct MyOtherError(i32, String);
```

#### Overriding `cause`

```rust
#[derive(Fail, Debug)]
extern crate failure;
#[macro_use] extern crate failure_derive;

use std::io;

/// MyError::cause will return a reference to the io_error field
#[derive(Fail, Debug)]
#[fail(display = "An error occurred.")]
struct MyError {
    #[cause] io_error: io::Error,
}

/// MyEnumError::cause will return a reference only if it is Variant2,
/// otherwise it will return None.
#[derive(Fail, Debug)]
enum MyEnumError {
    #[fail(display = "An error occurred.")]
    Variant1,
    #[fail(display = "A different error occurred.")]
    Variant2(#[cause] io::Error),
}
```

### The `Error` Type

In addition to the trait `Fail`, failure provides a type called `Error`. Any type that implements `Fail` can be cast into `Error` using From and Into, which allows users to throw errors using `?` which have different types, if the function returns an `Error`.

```rust
// Something you can deserialize
#[derive(Deserialize)]
struct Object {
    ...
}

impl Object {
    // This throws both IO Errors and JSON Errors, but they both get converted
    // into the Error type.
    fn from_file(path: &Path) -> Result<Object, Error> {
        let mut string = String::new();
        File::open(path)?.read_to_string(&mut string)?;
        let object = json::from_str(&string)?;
        Ok(object)
    }
}
```

## `KvStore`

下面梳理一下 `KvStore` 的结构与实现：

![image-20220729161125887](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220729161125887.png)

```rust
pub struct KvStore {
    // log 或者其他数据的目录
    path: PathBuf,
    // 由数字到读取文件的哈希表
    readers: HashMap<u64, BufReaderWithPos<File>>,
    // 当前 log 的 writer
    writer: BufWriterWithPos<File>,
    // 索引表，由 key 的 String 映射到命令的位置
    index: BTreeMap<String, CommandPos>,
    // 当前的日志标识
    current_gen: u64,
    // 在进行压缩时，可以删除的旧的命令的字节数
    uncompacted: u64,
}
```

为了高效存储，使用了日志结构来存储键值对。`current_gen` 用来标识不同的日志文件，其实就是数字，当新增加一个命令时，就创建一个日志文件。`readers` 为了高效的读取，会从日志的标识映射到内存中的 `BufReader`，但是为了方便找到位置，所以做了封装 `BufReaderWithPos`。为实现快速查找，内存中会有一个 `index` ，底层结构是 `BTreeMap` ，将日志标识映射到每一个日志条目（也就是一些命令）所在的位置。

如果日志数超过了一个阈值，会进行压缩，删除旧的不用的日志条目，合并成一个日志文件。这个值用 `uncompacted` 来记录。

如果执行如下命令：

```bash
cargo run --bin kvs -- set a a 
cargo run --bin kvs -- set b b 
cargo run --bin kvs -- set c c 
```

可以看到生成了三个日志文件，`1.log` 文件的内容就是命令序列化为 JSON 格式后的内容。

![image-20220729161556384](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220729161556384.png)

继续执行如下命令：

```bash
cargo run --bin kvs -- rm c 
run --bin kvs -- set a new_a
```

在代码中设置的阈值比较小：

```rust
const COMPACTION_THRESHOLD: u64 = 16;
```

执行完上述操作后，会对日志文件进行压缩，上面可以看到总共生成了4个日志文件，到第5个 `run --bin kvs -- set a new_a` 时，超过阈值了，需要进行压缩。临时的压缩日志号为6，接着更新当前的日志号为7。压缩结束后，会将所有日志号小于6的日志文件删除。这个压缩的过程在 `compact` 函数中实现。

```rust
/// 清除在日志 log 中旧的条目，进行压缩。
pub fn compact(&mut self) -> Result<()> {
    // 将 current_gen 增加 2
    // current_gen + 1 是为了给压缩文件用
    let compaction_gen = self.current_gen + 1;
    self.current_gen += 2;
    self.writer = self.new_log_file(self.current_gen)?;
    println!("compaction_gen: {}", compaction_gen);
    println!("current_gen: {}", self.current_gen);

    let mut compaction_writer = self.new_log_file(compaction_gen)?;

    let mut new_pos = 0; // 在新的 log 文件中的位置
    // 遍历所有日志中的命令，在 index 的 values 中
    for cmd_pos in &mut self.index.values_mut() {
        let reader = self
            .readers
            .get_mut(&cmd_pos.gen) // 得到当前命令的 gen 对应的 BufReaderWithPos<File>
            .expect("Cannot find log reader");
        if reader.pos != cmd_pos.pos {
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;
        }

        // Read trait 的 take 方法会返回一个 adapter ， 会有一个限制读的最大的字节参数
        let mut entry_reader = reader.take(cmd_pos.len);
        // io::copy 方法将一个 reader 中的内容复制到一个 writer 中
        let len = io::copy(&mut entry_reader, &mut compaction_writer)?;
        *cmd_pos = (compaction_gen, new_pos..new_pos + len).into(); // 更新 cmd_pos
        new_pos += len;
    }
    compaction_writer.flush()?;

    // 删除旧的日志文件
    let stale_gens: Vec<_> = self
        .readers
        .keys()
        .filter(|&&gen| gen < compaction_gen)
        .cloned()
        .collect();
    for stale_gen in stale_gens {
        self.readers.remove(&stale_gen);
        fs::remove_file(log_path(&self.path, stale_gen))?; // 删除文件
    }
    self.uncompacted = 0;

    Ok(())
}
```

这里输出了两个日志号的值，可以看到和分析的是一样的。

```bash
compaction_gen: 6
current_gen: 7
```

那么最终日志文件长什么样呢？

![image-20220729163332222](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220729163332222.png)

![image-20220729163349418](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220729163349418.png)

在`6.log` 中可以看到对旧的命令做了删除，而 `7.log`是空的。如果再执行一条命令 `cargo run --bin kvs -- set d d ` ，新的日志会顺序写入到 `8.log` 中。

![image-20220729163615078](/Users/jinjin/code/talent-plan-rust/rust/projects/img/image-20220729163615078.png)

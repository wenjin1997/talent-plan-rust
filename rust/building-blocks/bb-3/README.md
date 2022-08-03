# Notes -- Building Blocks 3

## `log` crate

什么是日志：

> 日志（Log）是系统运行过程中变化的一种抽象数据，其内容为指定对象的操作和其操作结果按时间的有序集合。

**Usage**

The basic use of the log crate is through the five logging macros: [`error!`](https://docs.rs/log/latest/log/macro.error.html), [`warn!`](https://docs.rs/log/latest/log/macro.warn.html), [`info!`](https://docs.rs/log/latest/log/macro.info.html), [`debug!`](https://docs.rs/log/latest/log/macro.debug.html) and [`trace!`](https://docs.rs/log/latest/log/macro.trace.html) where `error!` represents the highest-priority log messages and `trace!` the lowest. The log messages are filtered by configuring the log level to exclude messages with a lower priority. Each of these macros accept format strings similarly to [`println!`](https://doc.rust-lang.org/stable/std/macro.println.html).

**Implementing a Logger**

Loggers are installed by calling the [`set_logger`](https://docs.rs/log/latest/log/fn.set_logger.html) function. The maximum log level also needs to be adjusted via the [`set_max_level`](https://docs.rs/log/latest/log/fn.set_max_level.html) function. The logging facade uses this as an optimization to improve performance of log messages at levels that are disabled. It’s important to set it, as it defaults to [`Off`](https://docs.rs/log/latest/log/enum.LevelFilter.html), so no log messages will ever be captured! In the case of our example logger, we’ll want to set the maximum log level to [`Info`](https://docs.rs/log/latest/log/enum.LevelFilter.html), since we ignore any [`Debug`](https://docs.rs/log/latest/log/enum.Level.html) or [`Trace`](https://docs.rs/log/latest/log/enum.Level.html) level log messages. A logging implementation should provide a function that wraps a call to [`set_logger`](https://docs.rs/log/latest/log/fn.set_logger.html) and [`set_max_level`](https://docs.rs/log/latest/log/fn.set_max_level.html), handling initialization of the logger.

初始化时记得为 logger 设置 max_level。

**Use with `std`**

`set_logger` requires you to provide a `&'static Log`, which can be hard to obtain if your logger depends on some runtime configuration. The `set_boxed_logger` function is available with the `std` Cargo feature. It is identical to `set_logger` except that it takes a `Box<Log>` rather than a `&'static Log`:

```rust
pub fn init() -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(SimpleLogger))
        .map(|()| log::set_max_level(LevelFilter::Info))
}
```

**Crate Feature Flags**

The following crate feature flags are available in addition to the filters. They are configured in your `Cargo.toml`.

- `std` allows use of `std` crate instead of the default `core`. Enables using `std::error` and `set_boxed_logger` functionality.
- `serde` enables support for serialization and deserialization of `Level` and `LevelFilter`.

```rust
[dependencies]
log = { version = "0.4", features = ["std", "serde"] }
```

**Enum log::LevelFilter**

```rust
#[repr(usize)]
pub enum LevelFilter {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
```

## `Slog`

### [Slog - Structured, extensible, composable logging for Rust](https://docs.rs/slog/latest/slog/#slog----structured-extensible-composable-logging-for-rust)

`slog-rs` is an ecosystem of reusable components for structured, extensible, composable logging for Rust.

`slog` is `slog-rs`'s main crate providing core components shared between all other parts of `slog-rs` ecosystem.

This is auto-generated technical documentation of `slog`. For information about project organization, development, help, etc. please see [slog github page](https://github.com/slog-rs/slog)

### [Core advantages over `log` crate](https://docs.rs/slog/latest/slog/#core-advantages-over-log-crate)

* extensible
* componsable
* flexible
* **structured** and both **human and machine readable**
* **contextual**

### [Notable details](https://docs.rs/slog/latest/slog/#notable-details)

**Note:** At compile time `slog` by default removes trace and debug level statements in release builds, and trace level records in debug builds. This makes `trace` and `debug` level logging records practically free, which should encourage using them freely. If you want to enable trace/debug messages or raise the compile time logging level limit, use the following in your `Cargo.toml`:

```norust
slog = { version = ... ,
         features = ["max_level_trace", "release_max_level_warn"] }
```

Root drain (passed to `Logger::root`) must be one that does not ever return errors. This forces user to pick error handing strategy. `Drain::fuse()` or `Drain::ignore_res()`.

## Introduction to structured logging with slog

### Overview



### Drains

```rust
let file = File::create("/tmp/myloggingfile").unwrap();
let stream = slog_stream::stream(file, slog_json::new().build());
let syslog = slog_syslog::unix_3164(slog_syslog::Facility::LOG_DAEMON);
let root = Logger::root(Duplicate::new(
        LevelFilter::new(stream, Level::Info),
        LevelFilter::new(syslog, Level::Warning),
).fuse(), o!());
```

![image-20220801155759678](/rust/building-blocks/img/image-20220801155759678.png)

### Loggers

In `slog` any logging statement requires a `Logger` object.

The first `Logger` created will always have to be a root `Logger` using [`slog::Logger::root`](https://docs.rs/slog/*/slog/struct.Logger.html#method.root). Any other `Logger` objects can be build from the existing ones as it's child, using [`slog::Logger::new`](https://docs.rs/slog/*/slog/struct.Logger.html#method.new).

## [Benefits of Structured Logging vs basic logging](https://softwareengineering.stackexchange.com/questions/312197/benefits-of-structured-logging-vs-basic-logging)

如果日志是这样写的：

```
log.Debug("Disk quota {0} exceeded by user {1}", 100, "DTI-Matt");
log.Debug("Disk quota {0} exceeded by user {1}", 150, "nblumhardt");
```

那么最终会生成相似的文本：

```
Disk quota 100 exceeded by user DTI-Matt
Disk quota 150 exceeded by user nblumhardt
```

如果想要找到所有的 "disk quota exceeded" 事件，简单的 `like 'Disk quota%'` 查询会失败，因为可能找到这样的事件：

```
Disk quota 100 set for user DTI-Matt
```

文本的日志会丢失掉我们一开始拥有的事件的信息。相反地，我们可以这样来写：

```
log.Debug("Disk quota {Quota} exceeded by user {Username}", 100, "DTI-Matt");
log.Debug("Disk quota {Quota} exceeded by user {Username}", 150, "nblumhardt");
```

如果想要查找，可以用 `where MessageTemplate = 'Disk quota {Quota} exceeded by user {Username}'` 准确地找到。

还可以这样来结合查找：

```
Quota < 125 and EventType = 0x1234abcd
```

还有一些优点：

> One further benefit, perhaps not as easy to prevent up front, but once production debugging has been lifted out of the land of regex hackery, developers start to value logs a lot more and exercise more care and consideration when writing them. Better logs -> better quality applications -> more happiness all around.

## **[Reading: Redis Protocol specification](https://redis.io/topics/protocol)**

RESP：Redis serialization protocol  即Redis序列化协议。

RESP也可以用在其他的客户-服务端软件项目中。RESP可以将不同的数据类型序列化。

> RESP can serialize different data types like integers, strings, and arrays. There is also a specific type for errors. Requests are sent from the client to the Redis server as arrays of strings that represent the arguments of the command to execute. Redis replies with a command-specific data type.
>
> RESP is binary-safe and does not require processing of bulk data transferred from one process to another because it uses prefixed-length to transfer bulk data.

### Request-Response model

Redis accepts commands composed of different arguments. Once a command is received, it is processed and a reply is sent back to the client.

This is the simplest model possible; however, there are two exceptions:

- Redis supports pipelining (covered later in this document). So it is possible for clients to send multiple commands at once and wait for replies later.
- When a Redis client subscribes to a Pub/Sub channel, the protocol changes semantics and becomes a *push* protocol. The client no longer requires sending commands because the server will automatically send new messages to the client (for the channels the client is subscribed to) as soon as they are received.

除了上述的两种情况，Redis协议就是简单的 请求-回复 协议。

### RESP protocol description

RESP is actually a serialization protocol that supports the following data types: Simple Strings, Errors, Integers, Bulk Strings, and Arrays.

Redis uses RESP as a request-response protocol in the following way:

- Clients send commands to a Redis server as a RESP Array of Bulk Strings.
- The server replies with one of the RESP types according to the command implementation.

In RESP, the first byte determines the data type:

- For **Simple Strings**, the first byte of the reply is "+"
- For **Errors**, the first byte of the reply is "-"
- For **Integers**, the first byte of the reply is ":"
- For **Bulk Strings**, the first byte of the reply is "$"
- For **Arrays**, the first byte of the reply is "`*`"

In RESP, different parts of the protocol are always terminated with "\r\n" (CRLF).

### RESP Simple Strings[ ](https://redis.io/docs/reference/protocol-spec/#resp-simple-strings)

Simple Strings are used to transmit non binary-safe strings with minimal overhead. 

```
"+OK\r\n"
```

In order to send binary-safe strings, use RESP Bulk Strings instead.

### RESP Errors[ ](https://redis.io/docs/reference/protocol-spec/#resp-errors)

The real difference between Simple Strings and Errors in RESP is that clients treat errors as exceptions, and the string that composes the Error type is the error message itself.

```
"-Error message\r\n"
```

The following are examples of error replies:

```
-ERR unknown command 'helloworld'
-WRONGTYPE Operation against a key holding the wrong kind of value
```

The first word after the "-", up to the first space or newline, represents the kind of error returned. This is just a convention used by Redis and is not part of the RESP Error format.

### RESP Integers

For example, ":0\r\n" and ":1000\r\n" are integer replies.

However, the returned integer is guaranteed to be in the range of a signed 64-bit integer.

The following commands will reply with an integer: [`SETNX`](https://redis.io/commands/setnx), [`DEL`](https://redis.io/commands/del), [`EXISTS`](https://redis.io/commands/exists), [`INCR`](https://redis.io/commands/incr), [`INCRBY`](https://redis.io/commands/incrby), [`DECR`](https://redis.io/commands/decr), [`DECRBY`](https://redis.io/commands/decrby), [`DBSIZE`](https://redis.io/commands/dbsize), [`LASTSAVE`](https://redis.io/commands/lastsave), [`RENAMENX`](https://redis.io/commands/renamenx), [`MOVE`](https://redis.io/commands/move), [`LLEN`](https://redis.io/commands/llen), [`SADD`](https://redis.io/commands/sadd), [`SREM`](https://redis.io/commands/srem), [`SISMEMBER`](https://redis.io/commands/sismember), [`SCARD`](https://redis.io/commands/scard).

### RESP Bulk Strings

Bulk Strings are used in order to represent a single binary-safe string up to 512 MB in length.

So the string "hello" is encoded as follows:

```
"$5\r\nhello\r\n"
```

An empty string is encoded as:

```
"$0\r\n\r\n"
```

RESP Bulk Strings can also be used in order to signal non-existence of a value using a special format to represent a Null value. In this format, the length is -1, and there is no data. Null is represented as:

```
"$-1\r\n"
```

This is called a **Null Bulk String**.

### RESP Arrays

RESP Arrays are sent using the following format:

- A `*` character as the first byte, followed by the number of elements in the array as a decimal number, followed by CRLF.
- An additional RESP type for every element of the Array.

空数组

```
"*0\r\n"
```

含有两个Bulk String 的数组

```
"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
```

含有3个整数的数组

```
"*3\r\n:1\r\n:2\r\n:3\r\n"
```

拥有不同类型的数组

```
*5\r\n
:1\r\n
:2\r\n
:3\r\n
:4\r\n
$5\r\n
hello\r\n
```

Null Array

```
"*-1\r\n"
```

嵌套数组

```
*2\r\n
*3\r\n
:1\r\n
:2\r\n
:3\r\n
*2\r\n
+Hello\r\n
-World\r\n
```

### Null elements in Arrays

包含 Null 元素的数组

```
*3\r\n
$5\r\n
hello\r\n
$-1\r\n
$5\r\n
world\r\n
```

实际上就是类似这样的：

```
["hello",nil,"world"]
```

### Send commands to a Redis server

We can further specify how the interaction between the client and the server works:

- A client sends the Redis server a RESP Array consisting of only Bulk Strings.
- A Redis server replies to clients, sending any valid RESP data type as a reply.

The client sends the command **LLEN mylist** in order to get the length of the list stored at key *mylist*. Then the server replies with an Integer reply as in the following example (C: is the client, S: the server).

```
C: *2\r\n
C: $4\r\n
C: LLEN\r\n
C: $6\r\n
C: mylist\r\n

S: :48293\r\n
```

The actual interaction is the client sending `*2\r\n$4\r\nLLEN\r\n$6\r\nmylist\r\n` as a whole.

### Inline commands

有时候只需要给 Redis 服务器发送一个命令，但是只有 `telnet` 可用。由于 Redis 协议比较容易实现，所以用交互式的 session 不是很理想，并且 `redis-cli` 可能也不是总是能获取到的。由于这个原因，Redis 也允许 **inline command** 格式的命令。

```
C: PING
S: +PONG
```

```
C: EXISTS somekey
S: :0
```

### High performance parser for the Redis protocol

RESP uses prefixed lengths to transfer bulk data, so there is never a need to scan the payload for special characters, like with JSON, nor to quote the payload that needs to be sent to the server.

The Bulk and Multi Bulk lengths can be processed with code that performs a single operation per character while at the same time scanning for the CR character, like the following C code:

```c
#include <stdio.h>

int main(void) {
    unsigned char *p = "$123\r\n";
    int len = 0;

    p++;
    while(*p != '\r') {
        len = (len*10)+(*p - '0');
        p++;
    }

    /* Now p points at '\r', and the len is in bulk_len. */
    printf("%d\n", len);
    return 0;
}
```


# Notes -- Building Blocks 2

## LFS

下面记录了阅读论文 [Reading: The Design and Implementation of a Log-Structured File System](https://people.eecs.berkeley.edu/~brewer/cs262/LFS.pdf)的一些笔记：

Log-Structured File System 解决了哪些问题呢？

我们知道CPU的速度很快s但是磁盘读写的速度较慢。对于磁盘上的读操作，随着缓存容量的增加，比较好解决，出现速度限制的就是磁盘上的写操作。Log-Structured File System 就是考虑如何高效地进行磁盘的写操作，特别是一些小的文件的频繁写操作，比如办公和工程环境。

### LFS 数据结构

看看 Log-Structured File System 的一些结构：

![image-20220722111012181](/rust/building-blocks/img/image-20220722111012181.png)

### LFS 读取数据

比如下面这个例子，创建两个文件：

![image-20220722111327673](/rust/building-blocks/img/image-20220722111327673.png)

每个文件都有一个inode，存储该文件的一些属性。而 inode map 存储各个文件的 inode 信息。

The inode map is divided into blocks that are writ- ten to the log; a fixed checkpoint region on each disk identifies the locations of all the inode map blocks. For- tunately, inode maps are compact enough to keep the active portions cached in main memory: inode map lookups rarely require disk accesses.

### LFS 管理 Free Space

为了管理空闲空间，将磁盘分割成大小相等的段。处理空闲空间有两种方法：

![image-20220722112142882](/rust/building-blocks/img/image-20220722112142882.png)

LFS对此做了结合：

> Sprite LFS uses a combination of threading and copying. The disk is divided into large fixed-size extents called *segments*. Any given segment is always written sequentially from its beginning to its end, and all live data must be copied out of a segment before the segment can be rewritten. However, the log is threaded on a segment-by- segment basis; if the system can collect long-lived data together into segments, those segments can be skipped over so that the data doesn’t have to be copied repeatedly. The segment size is chosen large enough that the transfer time to read or write a whole segment is much greater than the cost of a seek to the beginning of the segment. This allows whole-segment operations to run at nearly the full bandwidth of the disk, regardless of the order in which seg- ments are accessed. Sprite LFS currently uses segment sizes of either 512 kilobytes or one megabyte.

总结起来就是先将磁盘分割成多个比较大的段。段里面写是顺序写入。如果想要重新写这个段，那么就把所有的 live date 写入复制到内存中，再写入一个可以写入的段中。接着这个段又可以被标记为可以再次利用的段了。

### **Segment cleaning policies**

文中对比几种方式进行了实验，最后考虑 cost-benefit policy。

>  The policy rates each segment according to the benefit of cleaning the segment and the cost of cleaning the segment and chooses the segments with the highest ratio of benefit to cost.  The benefit has two components: the amount of free space that will be reclaimed and the amount of time the space is likely to stay free. The amount of free space is just 1−*u*, where *u* is the utilization of the segment. We used the most recent modified time of any block in the segment (ie. the age of the youngest block) as an estimate of how long the space is likely to stay free. The benefit of cleaning is the space-time product formed by multiplying these two components. The cost of cleaning the segment is 1+*u* (one unit of cost to read the segment, *u* to write back the live data). Combining all these factors, we ges
>
> ![image-20220722113611940](/rust/building-blocks/img/image-20220722113611940.png)
>
> We call this policy the *cost-bene**fi**t* policy; it allows cold segments to be cleaned at a much higher utilization than hot segments.

为了实现这种方法，需要 segement usage table。

> In order to support the cost-benefit cleaning policy, Sprite LFS maintains a data structure called the *segment usage table*. For each segment, the table records the number of live bytes in the segment and the most recent modified time of any block in the segment. These two values are used by the segment cleaner when choosing seg- ments to clean. The values are initially set when the segment is written, and the count of live bytes is decremented when files are deleted or blocks are overwritten. If the count falls to zero then the segment can be reused without cleaning. The blocks of the segment usage table are writ- ten to the log, and the addresses of the blocks are stored in the checkpoint regions.

总结一下，segement usage table 记录了一个段中的两种数据：

* live bytes 的数量。当文件被删除或块被重写时，该值会减少。如果减少到0，不用 clean 这个段就可以重新使用了。
* 对于每一个块，记录最近修改的时间。

### **Crash recovery**

发生Crash不怕，有两种方式双管齐下：

* checkpoints 保证文件系统的一致性
* roll-forward 回滚

> Like many other logging systems, Sprite LFS uses a two-pronged approach to recovery: *checkpoints*, which define consistent states of the file system, and *roll-forward*, which is used to recover information written since the last checkpoint.
>

#### **Checkpoints**

> A checkpoint is a position in the log at which all of the file system structures are consistent and complete. Sprite LFS uses a two-phase process to create a checkpoint. First, it writes out all modified information to the log, including file data blocks, indirect blocks, inodes, and blocks of the inode map and segment usage table. Second, it writes a *checkpoint region* to a special fixed position on disk. The checkpoint region contains the addresses of all the blocks in the inode map and segment usage table, plus the current time and a pointer to the last segment written.
>

#### **Roll-forward**

> In order to recover as much information as possible, Sprite LFS scans through the log segments that were written after the last checkpoint. This operation is called *roll-forward*.

## Bitcask
Bitcask 可以达到如下的目标：
> • low latency per item read or written
>  • high throughput, especially when writing an incoming stream of random items
>  • ability to handle datasets much larger than RAM w/o degradation
>  • crash friendliness, both in terms of fast recovery and not losing data
>  • ease of backup and restore
>  • a relatively simple, understandable (and thus supportable) code structure and data format • predictable behavior under heavy access load or large volume
>  • a license that allowed for easy default use in Riak

它的内部结构是这样的，有 active data file，当这个写满时，就创建一个新的 active data file。

![image-20220724104116436](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104116436.png)

在 active data file 中数据都是顺序写入，这样顺序写入是不需要进行磁盘查找的。写入的数据格式是这样的：

![image-20220724104333987](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104333987.png)

那么在 active data file 中数据就是如下的线性结构：

![image-20220724104421157](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104421157.png)

当扩展完成的时候，存储中有"keydir"的结构会进行更新，实际上就是一个简单的哈希表。

![image-20220724104545671](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104545671.png)

当写入新的数据时，keydir会自动更新。要读取数据时，步骤如下：

![image-20220724104711940](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104711940.png)

由于是顺序写入，总会有将磁盘写满的时候，那怎么处理呢？merge process 会处理所有的 non-active 文件，并且产生一系列数据文件，只包含 “live” 和每个存在的键的最新版本的文件。

![image-20220724104830308](/Users/jinjin/code/talent-plan-rust/rust/building-blocks/img/image-20220724104830308.png)

注意到还有一个 hint 文件，它的作用是：

> When a Bitcask is opened by an Erlang process, it checks to see if there is already another Erlang process in the same VM that is using that Bitcask. If so, it will share the keydir with that process. If not, it scans all of the data files in a directory in order to build a new keydir. For any data file that has a hint file, that will be scanned instead for a much quicker startup time.

## `std::collections`
Rust’s collections can be grouped into four major categories:
* Sequences: Vec, VecDeque, LinkedList
* Maps: HashMap, BTreeMap
* Sets: HashSet, BTreeSet
* Misc: BinaryHeap

总结下每个怎么用：

* Vec: 集合
* VecDeque: 双端队列或者队列
* LinkedList：链表
* HashMap：哈希表
* BTreeMap：得到按照 key 排序的 map
* Sets：只考虑 key 的时候
* BinaryHeap：优先队列

看看一些方法的复杂度，`*`后缀表示均摊复杂度，`~`表示期望的复杂度，HashMaap的复杂度会取决于哈希值是如何设计计算的。

#### [Sequences](https://doc.rust-lang.org/std/collections/#sequences)

|                                                              | get(i)                 | insert(i)               | remove(i)              | append    | split_off(i)           |
| :----------------------------------------------------------- | :--------------------- | :---------------------- | :--------------------- | :-------- | :--------------------- |
| [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html)   | *O*(1)                 | *O*(*n*-*i*)*           | *O*(*n*-*i*)           | *O*(*m*)* | *O*(*n*-*i*)           |
| [`VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html) | *O*(1)                 | *O*(min(*i*, *n*-*i*))* | *O*(min(*i*, *n*-*i*)) | *O*(*m*)* | *O*(min(*i*, *n*-*i*)) |
| [`LinkedList`](https://doc.rust-lang.org/std/collections/struct.LinkedList.html) | *O*(min(*i*, *n*-*i*)) | *O*(min(*i*, *n*-*i*))  | *O*(min(*i*, *n*-*i*)) | *O*(1)    | *O*(min(*i*, *n*-*i*)) |

#### [Maps](https://doc.rust-lang.org/std/collections/#maps)

For Sets, all operations have the cost of the equivalent Map operation.

|                                                              | get           | insert        | remove        | range         | append       |
| :----------------------------------------------------------- | :------------ | :------------ | :------------ | :------------ | :----------- |
| [`HashMap`](https://doc.rust-lang.org/std/collections/hash_map/struct.HashMap.html) | *O*(1)~       | *O*(1)~*      | *O*(1)~       | N/A           | N/A          |
| [`BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) | *O*(log(*n*)) | *O*(log(*n*)) | *O*(log(*n*)) | *O*(log(*n*)) | *O*(*n*+*m*) |

#### [Correct and Efficient Usage of Collections](https://doc.rust-lang.org/std/collections/#correct-and-efficient-usage-of-collections)

In general, use `with_capacity` when you know exactly how many elements will be inserted, or at least have a reasonable upper-bound on that number.

When anticipating a large influx of elements, the `reserve` family of methods can be used to hint to the collection how much room it should make for the coming items. 

If you believe that a collection will not soon contain any more elements, or just really need the memory, the `shrink_to_fit` method prompts the collection to shrink the backing array to the minimum size capable of holding its elements.

Finally, if ever you’re interested in what the actual capacity of the collection is, most collections provide a `capacity` method to query this information on demand. This can be useful for debugging purposes, or for use with the `reserve` methods.

#### [Iterators](https://doc.rust-lang.org/std/collections/#iterators)

The three primary iterators almost every collection should provide are `iter`, `iter_mut`, and `into_iter`. Some of these are not provided on collections where it would be unsound or unreasonable to provide them.

`iter` provides an iterator of immutable references to all the contents of a collection in the most “natural” order. 

`iter_mut` provides an iterator of *mutable* references in the same order as `iter`. This is great for mutating all the contents of the collection.s

`into_iter` transforms the actual collection into an iterator over its contents by-value.

#### [Entries](https://doc.rust-lang.org/std/collections/#entries)

The `entry` API is intended to provide an efficient mechanism for manipulating the contents of a map conditionally on the presence of a key or not. 

When a user calls `map.entry(key)`, the map will search for the key and then yield a variant of the `Entry` enum.

If a `Vacant(entry)` is yielded, then the key *was not* found. 

If an `Occupied(entry)` is yielded, then the key *was* found. 

### Module [std](https://doc.rust-lang.org/std/index.html)::[io](https://doc.rust-lang.org/std/io/#)

重要的是 Read 和 Write 这两个 trait。

**[Seek and BufRead](https://doc.rust-lang.org/std/io/#seek-and-bufread)**

Beyond that, there are two important traits that are provided: [`Seek`](https://doc.rust-lang.org/std/io/trait.Seek.html) and [`BufRead`](https://doc.rust-lang.org/std/io/trait.BufRead.html). Both of these build on top of a reader to control how the reading happens.

**[BufReader and BufWriter](https://doc.rust-lang.org/std/io/#bufreader-and-bufwriter)**

`std::io` comes with two structs, [`BufReader`](https://doc.rust-lang.org/std/io/struct.BufReader.html) and [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html), which wrap readers and writers. The wrapper uses a buffer, reducing the number of calls and providing nicer methods for accessing exactly what you want.

**[Standard input and output](https://doc.rust-lang.org/std/io/#standard-input-and-output)**

 A very common source of output is standard output:

```rust
use std::io;
use std::io::prelude::*;

fn main() -> io::Result<()> {
    io::stdout().write(&[42])?;
    Ok(())
}
```

**[io::Result](https://doc.rust-lang.org/std/io/#ioresult)**

Last, but certainly not least, is [`io::Result`](https://doc.rust-lang.org/std/io/type.Result.html). This type is used as the return type of many `std::io` functions that can cause an error, and can be returned from your own functions as well. Many of the examples in this module use the [`?` operator](https://doc.rust-lang.org/book/appendix-02-operators.html):

```rust
use std::io;

fn read_input() -> io::Result<()> {
    let mut input = String::new();

    io::stdin().read_line(&mut input)?;

    println!("You typed: {}", input.trim());

    Ok(())
}
```

## 序列化

什么是序列化，看看[维基百科-序列化](https://zh.wikipedia.org/zh-my/%E5%BA%8F%E5%88%97%E5%8C%96)的定义：

> **序列化**（serialization）在[电脑科学](https://zh.wikipedia.org/wiki/計算機科學)的资料处理中，是指将[数据结构](https://zh.wikipedia.org/wiki/資料結構)或[对象](https://zh.wikipedia.org/wiki/物件_(计算机科學))状态转换成可取用格式（例如存成文件，存于缓冲，或经由网络中发送），以留待后续在相同或另一台电脑环境中，能恢复原先状态的过程。依照序列化格式重新获取[字节](https://zh.wikipedia.org/wiki/位元组)的结果时，可以利用它来产生与原始对象相同语义的副本。对于许多对象，像是使用大量[引用](https://zh.wikipedia.org/wiki/參照)的复杂对象，这种序列化重建的过程并不容易。面向对象中的对象序列化，并不概括之前原始对象所关系的函数。这种过程也称为对象编组（marshalling）。从一系列字节提取数据结构的反向操作，是反序列化（也称为解编组、deserialization、unmarshalling）。

Rust中处理序列化用 [serde](https://serde.rs/) 。serde 的设计如下：

> A data structure that knows how to serialize and deserialize itself is one that **implements Serde's `Serialize` and `Deserialize` traits** (or uses Serde's derive attribute to automatically generate implementations at compile time). 

### **Exercise: Serialize and deserialize a data structure with `serde` (JSON)**.

题目要求：

> This exercise and the next two will introduce basic serialization and deserialization with [`serde`](https://serde.rs/). `serde` serializes data quickly and is easy to use, while also being extensible and expressive.
>
> For your serializable data structure, imagine a flat game-playing surface covered in a grid of squares, like a chess board. Imagine you have a game character that every turn may move any number of squares in a single direction. Define a type, `Move` that represents a single move of that character.
>
> Derive the [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) trait so `Move` is easily printable with the `{:?}` format specifier.
>
> Write a `main` function that defines a variable, `a`, of type `Move`, serializes it with [`serde`](https://serde.rs/) to a [`File`](https://doc.rust-lang.org/std/fs/struct.File.html), then deserializes it back again to a variable, `b`, also of type `Move`.
>
> Use [JSON](https://github.com/serde-rs/json) as the serialization format.
>
> Print `a` and `b` with `println!` and the `{:?}` format specifier to verify successful deserialization.
>
> Note that the `serde` book has many [examples](https://serde.rs/examples.html) to work off of.

先说下什么是JSON：

> [JSON](https://github.com/serde-rs/json), the ubiquitous JavaScript Object Notation used by many HTTP APIs.

主要是对 [serde_json](https://github.com/serde-rs/json) 的使用。如果要用属性，不要忘记在 Cargo.toml 中加入属性项。

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

最终实现代码见[serde-json.rs](/rust/building-blocks/bb-2/serde-ex/src/bin/serde-json.rs)。

### **Exercise: Serialize and deserialize a data structure to a buffer with `serde` (RON)**.

什么是RON：

> [RON](https://github.com/ron-rs/ron), a Rusty Object Notation.

RON相比JSON的优势：

> Note the following advantages of RON over JSON:
>
> - trailing commas allowed
> - single- and multi-line comments
> - field names aren't quoted, so it's less verbose
> - optional struct names improve readability
> - enums are supported (and less verbose than their JSON representation)

最终实现代码见 [serde-ron.rs]()。

比较一下JSON格式和RON格式：

```
{"Up":56}  // JSON 格式
Up(56)     // RON 格式
```

### **Exercise: Serialize and deserialize 1000 data structures with `serde` (BSON)**.

什么是BSON：

> - [BSON](https://github.com/mongodb/bson-rust), the data storage and network transfer format used by MongoDB.

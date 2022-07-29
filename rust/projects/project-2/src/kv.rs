use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::{fs, io};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use crate::{KvError, Result};

/// 压缩的阈值
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

/// `KvStore` 存储的是字符串的键-值对。
///
/// key/value 键-值对通过日志文件保存到磁盘中。日志文件名
/// 是一个 generation 数递增得到的，日志文件后缀是`.log`。
/// 内存中有一个 `BTreeMap` 结构来存储键和值所在的位置，
/// 方便快速查找。
///
/// ```rust
/// # use kvs::{KvStore, Result};
/// # fn try_main() -> Result<()> {
///  use std::env::current_dir;
///  let mut store = KvStore::open(current_dir()?)?;
///  store.set("key".to_owned(), "value".to_owned())?;
///  let val = store.get("key".to_owned())?;
///  assert_eq!(val, Some("value".to_owned()));
/// # Ok(())
/// # }
/// ```
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

impl KvStore {
    /// 通过给定的路径 path 打开一个 `KvStore`。
    ///
    /// 如果给定的路径 path 不存在会创建一个新的目录。
    ///
    /// # Errors
    ///
    /// 会传播 I/O 或者解序列化错误。
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into(); // 先将 path 转为 PathBuf
        fs::create_dir_all(&path)?;     // 创建目录，类似命令 `mkdir` ，如果文件目录创建不了会产生错误

        let mut readers = HashMap::new();
        let mut index = BTreeMap::new();

        // 将 path 加入到 gen_list 中并进行排序，返回 gen_list
        // gen_list 为 Vec::<u64>
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }

        // 下一个 gen 要在当前的最后的一个基础上 + 1
        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        Ok(KvStore {
            path,
            readers,
            writer,
            current_gen,
            index,
            uncompacted,
        })
    }

    /// 给一个字符串的 key 设置字符串的 value。
    ///
    /// 如果 key 已经存在，之前的值会被重写。
    ///
    /// # Errors
    ///
    /// 在写入到日志中会传播 I/O 或者序列化的错误。
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let cmd = Command::set(key, value);
        let pos = self.writer.pos;
        // to_writer 方法将给定的数据结构作为 JSON 写入到 IO 流中
        serde_json::to_writer(&mut self.writer, &cmd)?;
        self.writer.flush()?; // 确保所有在缓存中的数据到达目的地
        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self
                .index
                .insert(key, (self.current_gen, pos..self.writer.pos).into())
            {
                self.uncompacted += old_cmd.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }

    /// 得到给定字符串键 key 对应的 value 值。
    ///
    /// 如果所给的键不存在，则返回 `None`。
    ///
    /// # Errors
    ///
    /// 如果所给的命令类型不是期望的，返回 `KvError::UnexpectedCommandType`。
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            let cmd_reader = reader.take(cmd_pos.len);
            if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                Ok(Some(value))
            } else {
                Err(KvError::UnexpectedCommandType)
            }
        } else {
            Ok(None)
        }
    }

    /// 删除一个给定的键。
    ///
    /// # Errors
    ///
    /// 如果给定的键不存在，返回 `KvError::KeyNotFound`。
    ///
    /// 在写到 log 日志中的过程中会传播 I/O 或者序列化的错误。
    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let cmd = Command::remove(key);
            serde_json::to_writer(&mut self.writer, &cmd)?;
            self.writer.flush()?;
            if let Command::Remove { key } = cmd {
                let old_cmd = self.index.remove(&key).expect("key not found");
                self.uncompacted += old_cmd.len;
            }
            Ok(())
        } else {
            Err(KvError::KeyNotFound)
        }
    }

    /// 清除在日志 log 中旧的条目，进行压缩。
    pub fn compact(&mut self) -> Result<()> {
        // 将 current_gen 增加 2
        // current_gen + 1 是为了给压缩文件用
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;
        self.writer = self.new_log_file(self.current_gen)?;
        // println!("compaction_gen: {}", compaction_gen);
        // println!("current_gen: {}", self.current_gen);

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

    fn new_log_file(&mut self, gen: u64) -> Result<BufWriterWithPos<File>> {
        new_log_file(&self.path, gen, &mut self.readers)
    }
}

/// 在给定的目录中返回排序后的 generation 数值。
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)? // Returns an iterator over the entries within a directory.
        .flat_map(|res| -> Result<_> { Ok(res?.path()) }) // 返回 PathBuf
        // 选择那些 path 是文件并且扩展名为 ".log" 的
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        // 找到文件名，转换为字符串，删除后缀名".log"，然后转成 u64
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))// 匹配并删除后缀".log"
                .map(str::parse::<u64>)
        })
        // 将所有这些符合的 u64 合并成一个 iterator
        .flatten()
        .collect(); // 转成集合 Vec<u64>

    // 对 gen_list 进行快速排序，unstable 是对于相同的元素，可能不会保持原来的顺序
    gen_list.sort_unstable();
    Ok(gen_list)
}

/// 加载整个 log 文件，并且将值所在的位置存在 index map 中。
///
/// 返回在压缩之后还能存多少字节。
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &mut BTreeMap<String, CommandPos>
) -> Result<u64> {
    // 确保从文件的起始位置读
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    // 反序列化
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
    let mut uncompacted = 0; // 在压缩之后还能存放的字节数
    while let Some(cmd) = stream.next() {
        // byte_offset() 返回到目前为止反序列化成功的字节数
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                // 如果有旧的值，就更新
                if let Some(old_cmd) = index.insert(key, (gen, pos..new_pos).into()) {
                    uncompacted += old_cmd.len;
                }
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                }
                // "remove" 命令可以在下次压缩时被删除
                // 所以我们将它的长度加到 `uncompacted` 中
                uncompacted += new_pos - pos;
            }
        }
        pos = new_pos;
    }
    Ok(uncompacted)
}


/// 通过给定的 generation 数创建一个新的 log 文件并且将 reader 加到 readers map 中。
///
/// 返回 log 的 writer。
fn new_log_file(
    path: &Path,
    gen: u64,
    readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    // 生成日志文件
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    readers.insert(gen, BufReaderWithPos::new(File::open(&path)?)?);
    Ok(writer)
}


/// 生成带有 gen 的日志文件。
///
/// 返回 PathBuf。
fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// 表示命令的一个结构体。
///
/// 命令有：set 和 remove，可以写进日志中。
///
/// 而 get 命令可以直接读取，不需要写入。
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

// Command 的两个方法，方便生成 Command 实例
impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

/// 代表在日志中一个 json 序列化的命令的位置和长度。
struct CommandPos {
    gen: u64,
    pos: u64,
    len: u64,
}

// 方便从 (gen, range) 转换到 CommandPos
impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        CommandPos {
            gen,
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

/// 带有位置的 BufReader。
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

// 构造 BufReaderWithPos 实例的方法
impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        // 在流中查找位置，将偏移量设置为当前位置
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    /// 将资源读到缓存中，返回已经读了多少字节。
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        // 更新 BufReaderWithPos 的位置
        self.pos += len as u64;
        Ok(len)
    }
}

// Seek trait 提供了一个可以在字节流中移动的光标
// 流通常具有固定的大小，因此可以对流的任意一端或者当前偏移位置进行搜索
impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    /// seek 方法在流中寻找偏移量，以字节为单位。
    ///
    /// 如果查找成功，从流的开头返回新位置。
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

/// 带有位置的 BufWriter 结构体。
///
/// 由于需要定位，因此要绑定 Seek trait。
struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

// 实现 Write trait 必须要实现 write 和 flush 这两个方法
impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    /// write 方法尝试缓存中的数据写入 writer ，返回成功写入的字节数。
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    /// flush 方法保证缓冲区的所有数据到达目的地。
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
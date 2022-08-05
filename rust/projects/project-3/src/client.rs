use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};
use crate::common::{SetResponse, GetResponse, RemoveResponse, Request};
use crate::{KvsError, Result};
use serde::{Deserialize};

// serde_json: 反序列化 JSON 数据格式到 Rust 数据结构。
// IoRead: JSON input source that reads from a std::io input stream.
use serde_json::de::{Deserializer, IoRead};

/// 键值对存储的客户端。
pub struct KvsClient {
    // serde_json::de::Deserializer
    //      A structure that deserializes JSON into Rust values.
    //      pub struct Deserializer<R>
    //
    // serde_json::de::IoRead
    //      JSON input source that reads from a std::io input stream.
    //      pub struct IoRead<R>
    //      where
    //          R: io::Read,
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {
    /// 连接到 `addr` 以访问 `KvsServer` 。
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;
        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// 通过服务器得到一个给定 key 的 value。
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        serde_json::to_writer(&mut self.writer, &Request::Get { key })?;
        self.writer.flush()?;
        let resp = GetResponse::deserialize(&mut self.reader)?;
        match resp {
            GetResponse::Ok(value) => Ok(value),
            GetResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }

    /// 在服务器中给一个字符串的 key 设置一个字符串的 value。
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Set { key, value })?;
        self.writer.flush()?;
        let resp = SetResponse::deserialize(&mut self.reader)?;
        match resp {
            SetResponse::Ok(_) => Ok(()),
            SetResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }

    /// 在服务器中删除一个给定的字符串 key。
    pub fn remove(&mut self, key: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Remove { key })?;
        self.writer.flush()?;
        let resp = RemoveResponse::deserialize(&mut self.reader)?;
        match resp {
            RemoveResponse::Ok(_) => Ok(()),
            RemoveResponse::Err(msg) => Err(KvsError::StringError(msg)),
        }
    }
}
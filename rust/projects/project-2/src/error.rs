use failure::Fail;
use std::io;
use std::result;

/// 定义 `KvStore` 的错误类型。
#[derive(Debug, Fail)]
#[fail(display = "Kv Error")]
pub enum KvError {
    /// 未找到 key。
    #[fail(display= "Key Not Found")]
    KeyNotFound,

    /// 序列化错误。
    #[fail(display = "Serde Error: {}", _0)]
    SerdeError(#[cause] serde_json::Error),

    /// I/O 错误。
    #[fail(display = "IO error: {}", _0)]
    IoError(#[cause] io::Error),

    /// 命令错误。
    #[fail(display = "Unexpected Command Error")]
    UnexpectedCommandType,
}

impl From<io::Error> for KvError {
    fn from(error: io::Error) -> Self {
        KvError::IoError(error)
    }
}

impl From<serde_json::Error> for KvError {
    fn from(error: serde_json::Error) -> Self {
        KvError::SerdeError(error)
    }
}

/// kvs 的 Result 类型。
pub type Result<T> = result::Result<T, KvError>;

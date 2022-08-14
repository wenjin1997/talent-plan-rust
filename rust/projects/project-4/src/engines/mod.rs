//! 这个模块提供了几种 key value 存储引擎。

use crate::Result;

/// 一个 key value 存储引擎的 trait。
pub trait KvsEngine: Clone + Send + 'static {
    /// 给一个字符串 key 设置字符串的 value。
    ///
    /// 如果 key 已经存在，之前的值会被重写。
    fn set(&self, key: String, value: String) -> Result<()>;

    /// 得到给定字符串键 key 对应的 value 值。
    ///
    /// 如果所给的键不存在，则返回 `None`。
    fn get(&self, key: String) -> Result<Option<String>>;

    /// 删除一个给定的键。
    ///
    /// # Errors
    ///
    /// 如果给定的键不存在，返回 `KvsError::KeyNotFound`。
    fn remove(&self, key: String) -> Result<()>;
}


mod kvs;
mod sled;

pub use self::kvs::KvStore;
pub use self::sled::SledKvsEngine;
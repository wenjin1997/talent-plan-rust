use super::KvsEngine;
use crate::{KvsError, Result};
use sled::{Db, Tree};

/// `sled::Db` 的 wrapper。
#[derive(Clone)]
pub struct SledKvsEngine(Db);

impl SledKvsEngine {
    /// 通过 `sled::Db` 创建一个 `SledKvsEngine`。
    pub fn new (db: Db) -> Self {
        SledKvsEngine(db)
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        // 因为 `sled::Db` 实现了：
        // Implements Deref<Target = sled::Tree> to refer to
        // a default keyspace / namespace / bucket.
        let tree: &Tree = &self.0;
        tree.insert(key, value.into_bytes()).map(|_| ())?;
        tree.flush()?;
        Ok(())
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        let tree: &Tree = &self.0;
        Ok(tree
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            // transpose() 函数： Transposes an Option of a Result into a Result of an Option.
            // 将 Option<Result<T, E>> 转换成 Result<Option<T>, E>
            .transpose()?)
    }

    fn remove(&mut self, key: String) -> Result<()> {
        let tree: &Tree = &self.0;
        // Option 中的 ok_or 函数：
        // Transforms the Option<T> into a [Result<T, E>],
        // mapping Some(v) to Ok(v) and None to Err(err).
        tree.remove(key)?.ok_or(KvsError::KeyNotFound)?;
        // flush 函数：
        // Synchronously flushes all dirty IO buffers and calls fsync.
        tree.flush()?;
        Ok(())
    }
}
use std::collections::HashMap;

/// `KvStore` 存储的是 key/value 键值对。
///
/// 键值对存储在一个 `HashMap` 中。
///
/// Example:
///
/// ```rust
/// # use kvs::KvStore;
/// let mut store = KvStore::new();
/// store.set("key".to_owned(), "value".to_owned());
/// assert_eq!(store.get("key".to_owned()), Some("value".to_owned()));
/// ```
#[derive(Default)]
pub struct KvStore {
    db: HashMap<String, String>,
}

impl KvStore {
    /// 创建一个 `KvStore`
    pub fn new() -> KvStore {
        KvStore { db: HashMap::new() }
    }

    /// 设置一个字符串的键值对
    ///
    /// 如果一个键已经存在，则会覆盖之前的值
    pub fn set(&mut self, key: String, value: String) {
        self.db.insert(key, value);
    }

    /// 得到一个字符串的键对应的值
    ///
    /// 如果键对应的值不存在，返回 `None`
    pub fn get(&self, key: String) -> Option<String> {
        self.db.get(&key).cloned()
    }

    /// 移除一个给定的键
    pub fn remove(&mut self, key: String) {
        self.db.remove(&key);
    }
}

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Db {
    pub data: Arc<Mutex<HashMap<String, Bytes>>>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert(&self, key: String, value: Bytes) {
        let mut db = self.data.lock().unwrap();
        db.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        let db = self.data.lock().unwrap();
        db.get(key).cloned()
    }
}

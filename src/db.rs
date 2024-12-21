use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

pub type Db = Arc<Mutex<HashMap<String, Bytes>>>;

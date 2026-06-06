use std::{collections::HashMap, sync::Mutex};

use janus::StringHash;

pub type StringMap = HashMap<StringHash, &'static str, janus::StringHasher>;

static REGISTRY: Mutex<StringMap> = Mutex::new(StringMap::with_hasher(janus::StringHasher::new()));

#[macro_export]
macro_rules! lazy_hash_str {
    ($sl:literal) => {
        std::cell::LazyCell::new(|| $crate::assets::strings::hash($sl))
    };
    ($se:expr) => {
        std::cell::LazyCell::new(|| $crate::assets::strings::hash($se))
    };
}

pub fn hash(string: &'static str) -> StringHash {
    let hashed = janus::hash_string(string);
    REGISTRY.lock().unwrap().insert(hashed, string);
    hashed
}

pub fn fetch(hash: &StringHash) -> Option<&'static str> {
    REGISTRY.lock().unwrap().get(hash).copied()
}

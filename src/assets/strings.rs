use std::{collections::HashMap, sync::RwLock};

use janus::{StringHash, StringHasher};

pub type StringMap = HashMap<StringHash, &'static str, StringHasher>;

static REGISTRY: RwLock<StringMap> = RwLock::new(StringMap::with_hasher(StringHasher::new()));

#[macro_export]
macro_rules! lazy_hash_str {
    ($sl:literal) => {
        std::sync::LazyLock::new(|| $crate::assets::strings::hash($sl))
    };
    ($se:expr) => {
        std::sync::LazyLock::new(|| $crate::assets::strings::hash($se))
    };
}

pub fn hash(string: &'static str) -> StringHash {
    let hashed = janus::hash_string(string);
    REGISTRY.write().unwrap().insert(hashed, string);
    hashed
}

pub fn fetch(hash: &StringHash) -> Option<&'static str> {
    REGISTRY.read().unwrap().get(hash).copied()
}

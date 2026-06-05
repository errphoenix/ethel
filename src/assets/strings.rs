use std::{collections::HashMap, sync::Mutex};

use janus::StringHash;

pub type StringMap = HashMap<StringHash, &'static str, janus::StringHasher>;

static REGISTRY: Mutex<StringMap> = Mutex::new(StringMap::with_hasher(janus::StringHasher::new()));

pub fn hash(string: &'static str) -> StringHash {
    let hashed = janus::hash_string(string);
    REGISTRY.lock().unwrap().insert(hashed, string);
    hashed
}

pub fn fetch(hash: &StringHash) -> Option<&'static str> {
    REGISTRY.lock().unwrap().get(hash).copied()
}

// maybe support const by lazy initialization of registry

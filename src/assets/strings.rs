use std::{collections::HashMap, sync::RwLock};

use janus::{StringHash, StringHasher};

pub type StringCache = HashMap<StringHash, &'static str, StringHasher>;

static REGISTRY: RwLock<StringCache> = RwLock::new(StringCache::with_hasher(StringHasher::new()));

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CachedStringHash(StringHash);
impl CachedStringHash {
    pub const fn inner(self) -> StringHash {
        self.0
    }
}
impl From<CachedStringHash> for StringHash {
    fn from(value: CachedStringHash) -> Self {
        value.0
    }
}

#[macro_export]
macro_rules! lazy_hash_str {
    ($($pv:vis $pname:ident = $psl:literal;)+) => {
        paste::paste! {
            $(
                $pv const [< $pname:upper >]: std::sync::LazyLock<$crate::assets::strings::CachedStringHash> = $crate::lazy_hash_str!($psl);
            )+
        }
    };
    ($pv:vis $pname:ident = $psl:literal;) => {
        paste::paste! {
            $pv const [< $pname:upper >]: std::sync::LazyLock<$crate::assets::strings::CachedStringHash> = $crate::lazy_hash_str!($psl);
        }
    };
    ($pv:vis $pname:ident = $pse:literal;) => {
        paste::paste! {
            $pv const [< $pname:upper >]: std::sync::LazyLock<$crate::assets::strings::CachedStringHash> = $crate::lazy_hash_str!($pse);
        }
    };
    ($sl:literal) => {
        std::sync::LazyLock::new(|| $crate::assets::strings::hash($sl))
    };
    ($se:expr) => {
        std::sync::LazyLock::new(|| $crate::assets::strings::hash($se))
    };
}

pub fn hash(string: &'static str) -> CachedStringHash {
    let hashed = janus::hash_string(string);
    REGISTRY.write().unwrap().insert(hashed, string);
    CachedStringHash(hashed)
}

pub fn fetch(hash: CachedStringHash) -> &'static str {
    REGISTRY
        .read()
        .unwrap()
        .get(&hash.0)
        .copied()
        .expect("CachedStringHash is always present in string cache")
}

pub fn fetch_raw_hash(hash: StringHash) -> Option<&'static str> {
    REGISTRY.read().unwrap().get(&hash).copied()
}

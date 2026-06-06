use janus::StringHash;

pub mod strings;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetId(pub StringHash);

impl AssetId {
    pub const fn hash(&self) -> StringHash {
        self.0
    }
}

#[macro_export]
macro_rules! hashet {
    ($se:expr) => {
        std::cell::LazyCell::new(|| {
            let hashed = $crate::lazy_hash_str!($se);
            $crate::assets::AssetId(*hashed)
        })
    };
    ($sl:literal) => {
        std::cell::LazyCell::new(|| {
            let hashed = $crate::lazy_hash_str!($sl);
            $crate::assets::AssetId(*hashed)
        })
    };
}


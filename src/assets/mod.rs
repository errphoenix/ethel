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
    ($s:literal) => {
        $crate::assets::AssetId($crate::assets::strings::hash(s))
    };
}

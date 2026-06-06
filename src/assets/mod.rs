use std::{collections::HashMap, path::Path};

use janus::{
    StringHash,
    texture::{Texture, TextureView},
};

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

pub trait ImportAsset<T: HasAsset> {
    /// Import the asset from a file `path` or panic.
    fn import_from_file<P: AsRef<Path>>(path: P) -> T {
        Self::try_import_from_file(path)
            .expect("unexpected file open error when trying to read file path {path}")
    }

    /// Attempts to import the asset from a file `path`.
    ///
    /// This wraps a call to [`ImportAsset::import`].
    ///
    /// # Returns
    /// This function will return an error according to
    /// [`std::fs::OpenOptions::open`].
    fn try_import_from_file<P: AsRef<Path>>(path: P) -> Result<T, std::io::Error> {
        std::fs::read(path).map(|bytes| Self::import(&bytes))
    }

    fn import(bytes: &[u8]) -> T;
}

#[derive(Debug, Eq)]
pub struct RawAsset<T> {
    id: AssetId,
    inner: T,
}

impl<T> PartialEq for RawAsset<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> RawAsset<T> {
    pub const fn new(id: AssetId, value: T) -> Self {
        Self { id, inner: value }
    }

    pub const fn id(&self) -> AssetId {
        self.id
    }

    pub const fn asset(&self) -> &T {
        &self.inner
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub AssetId);

pub trait HasAsset {}

pub trait ViewAsset<V: Clone + Copy>: HasAsset {
    fn view(&self) -> V;
}

pub trait ManageAssets<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
    V: HasAsset,
{
    fn inner_map(&self) -> &HashMap<K, V>;

    fn inner_map_mut(&mut self) -> &mut HashMap<K, V>;

    fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner_map().iter()
    }

    fn take(&mut self, id: &K) -> Option<V> {
        self.inner_map_mut().remove(id)
    }

    fn store(&mut self, id: K, asset: V) {
        self.inner_map_mut().insert(id, asset);
    }

    fn contains(&self, id: &K) -> bool {
        self.inner_map().contains_key(id)
    }

    fn get<'v>(&'v self, id: &'v K) -> Option<&'v V> {
        self.inner_map().get(id)
    }
}

pub trait ManageViews<K, V, L>: ManageAssets<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
    L: Clone + Copy,
    V: HasAsset + ViewAsset<L>,
{
    fn view(&self, id: &K) -> Option<L> {
        self.inner_map().get(id).map(|asset| V::view(asset))
    }
}

impl<T, K, V, L> ManageViews<K, V, L> for T
where
    K: std::cmp::Eq + std::hash::Hash,
    L: Clone + Copy,
    V: HasAsset + ViewAsset<L>,
    T: ManageAssets<K, V>,
{
}

#[derive(Debug, Default)]
pub struct TextureManager {
    map: HashMap<TextureId, Texture>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }
}

impl ManageAssets<TextureId, Texture> for TextureManager {
    fn inner_map(&self) -> &HashMap<TextureId, Texture> {
        &self.map
    }

    fn inner_map_mut(&mut self) -> &mut HashMap<TextureId, Texture> {
        &mut self.map
    }
}

impl HasAsset for Texture {}

impl ViewAsset<TextureView> for Texture {
    fn view(&self) -> TextureView {
        Texture::view(&self)
    }
}

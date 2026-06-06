use std::{collections::HashMap, error::Error, fmt::Debug, path::Path};

use image::DynamicImage;
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

pub trait ProcessResource<T: HasAsset> {
    fn process_resource(&self) -> Option<T>;
}

pub trait Import<E: Error + 'static> {
    fn import_from_dir<P: AsRef<Path> + Debug>(path: P, recursive: bool) -> Vec<Self>
    where
        Self: Sized,
    {
        Self::try_import_from_dir(&path, recursive).expect(&format!(
            "unexpected error while importing assets from dir {path:?} [recursive: {recursive}]"
        ))
    }

    fn try_import_from_dir<P: AsRef<Path>>(
        path: P,
        recursive: bool,
    ) -> Result<Vec<Self>, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let dir = std::fs::read_dir(path)?;
        let mut buffer = Vec::new();

        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                if !recursive {
                    continue;
                }

                let from_dir = Self::try_import_from_dir(path, true)?;
                buffer.extend(from_dir);
            } else {
                let from_file = Self::try_import_from_file(path)?;
                buffer.push(from_file);
            }
        }

        Ok(buffer)
    }

    /// Import the asset from a file `path` or panic.
    fn import_from_file<P: AsRef<Path>>(path: P) -> Self
    where
        Self: Sized,
    {
        let bytes = std::fs::read(path)
            .expect("unexpected file open error when trying to read file path {path}");

        Self::import(&bytes)
    }

    /// Attempts to import the asset from a file `path`.
    ///
    /// This wraps a call to [`ImportAsset::import`].
    ///
    /// # Returns
    /// This function will return an error according to
    /// [`std::fs::OpenOptions::open`] and [`image::ImageReader::open`].
    fn try_import_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let bytes = std::fs::read(path)?;
        let result = Self::try_import(&bytes)?;
        Ok(result)
    }

    fn import(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        Self::try_import(bytes).expect("unexpected import error")
    }

    fn try_import(bytes: &[u8]) -> Result<Self, E>
    where
        Self: Sized;
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub AssetId);

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

#[derive(Clone, Debug, Default)]
pub struct RawTexture(DynamicImage);

impl RawTexture {
    pub const fn new(image: DynamicImage) -> Self {
        Self(image)
    }

    pub const fn image(&self) -> &DynamicImage {
        &self.0
    }
}

impl ProcessResource<Texture> for RawTexture {
    fn process_resource(&self) -> Option<Texture> {
        assert!(janus::gl::has_gl_init());
        Some(
            Texture::from_image(&self.0)
                .expect("cannot process texture resource: unsupported image format"),
        )
    }
}

impl Import<image::ImageError> for RawTexture {
    fn try_import(bytes: &[u8]) -> Result<Self, image::ImageError>
    where
        Self: Sized,
    {
        image::load_from_memory(bytes).map(RawTexture::new)
    }
}

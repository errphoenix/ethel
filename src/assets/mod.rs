use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

use image::DynamicImage;
use janus::{
    GpuResource, StringHash,
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

impl Into<StringHash> for AssetId {
    fn into(self) -> StringHash {
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

#[derive(Debug, Default)]
pub struct AssetRegistry<T> {
    assets: HashMap<StringHash, T, janus::StringHasher>,
}

impl<T> AssetRegistry<T> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::with_hasher(janus::StringHasher::default()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            assets: HashMap::with_capacity_and_hasher(capacity, janus::StringHasher::default()),
        }
    }

    pub fn insert(&mut self, id: impl Into<StringHash>, asset: T) {
        self.assets.insert(id.into(), asset);
    }

    pub fn remove(&mut self, id: impl Into<StringHash>) -> Option<T> {
        self.assets.remove(&id.into())
    }

    pub fn get(&self, id: impl Into<StringHash>) -> Option<&T> {
        self.assets.get(&id.into())
    }
}

pub trait AsView {
    type View;

    fn as_view(&self) -> Self::View;
}

impl<T: AsView> AssetRegistry<T> {
    pub fn get_view(&self, id: impl Into<StringHash>) -> Option<T::View> {
        self.assets.get(&id.into()).map(AsView::as_view)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ResourceState {
    #[default]
    Unloaded,
    InMemory,
    Processed,
}

impl std::fmt::Display for ResourceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceState::Unloaded => write!(f, "UNLOADED"),
            ResourceState::InMemory => write!(f, "IN RAM"),
            ResourceState::Processed => write!(f, "PROCESSED (GPU)"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("invalid resource state for requested operation: expected {expected}, got {present}.")]
    InvalidState {
        present: ResourceState,
        expected: ResourceState,
    },

    #[error("failed to load: file not found at path {0}")]
    FileNotFound(PathBuf),

    #[error("failed to load due to a file io error: {0}")]
    FileIoError(std::io::Error),

    #[error("failed to load image from memory: {0}")]
    FileImageLoadError(image::ImageError),

    #[error("failed to process resource for gpu: gl context unavailable")]
    NoGlContext,
}

pub type AssetResult<T> = Result<T, AssetError>;

#[derive(Debug)]
pub struct Handle<T>
where
    T: Import + IntoGpu,
{
    state: ResourceState,
    source: PathBuf,
    raw_resource: Option<T>,
    gpu_resource: Option<T::AsGpu>,
}

macro_rules! assert_state {
    ($s:ident, $state:expr) => {
        if $s.state != $state {
            return Err(AssetError::InvalidState {
                present: $s.state,
                expected: $state,
            });
        }
    };
}

impl<T> Handle<T>
where
    T: Import + IntoGpu,
{
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            source: path.as_ref().to_path_buf(),
            state: ResourceState::Unloaded,
            raw_resource: None,
            gpu_resource: None,
        }
    }

    pub const fn state(&self) -> ResourceState {
        self.state
    }

    pub fn file_source(&self) -> &Path {
        &self.source
    }

    pub fn raw_resource(&self) -> Option<&T> {
        self.raw_resource.as_ref()
    }

    pub fn gpu_resource(&self) -> Option<&T::AsGpu> {
        self.gpu_resource.as_ref()
    }

    /// Attempt to load the raw resource from disk.
    ///
    /// This operation must only occur during the [`ResourceState::Unloaded`]
    /// state.
    ///
    /// # Returns
    /// The operation might return any of these errors under a specific
    /// condition:
    /// * [`InvalidState`] if the current state of the asset does not correpond
    ///   to [`ResourceState::unloaded`].
    /// * [`FileNotFound`] if this asset's `path` could not be located on the
    ///   file system.
    /// * [`FileIoError`] wrapping a [`std::io::Error`] type for any other IO
    ///   as according to [`std::fs::OpenOptions::open`].
    /// * [`FileImageLoadError`] wrapping a [`image::ImageError`] type for any
    ///   image decode error as according to [`image::ImageDecoder::decode`].
    ///
    /// If the operation is successful, a borrow to the `C` raw resource that
    /// was just loaded is returned.
    ///
    /// [`AssetError::InvalidState`]: InvalidState
    /// [`AssetError::FileNotFound`]: FileNotFound
    /// [`AssetError::FileIoError`]: FileIoError
    /// [`AssetError::FileImageLoadError`]: FileImageLoadError
    pub fn load_to_memory(&mut self) -> AssetResult<&T> {
        assert_state!(self, ResourceState::Unloaded);

        let path = &self.source;
        if !path.is_file() {
            return Err(AssetError::FileNotFound(path.to_path_buf()));
        }

        let loaded = T::from_file(path)?;

        self.raw_resource = Some(loaded);
        self.state = ResourceState::InMemory;

        Ok(self.raw_resource.as_ref().unwrap())
    }

    /// Attempt to load the resource to the gpu.
    ///
    /// This operation must be called on the graphics/windowing thread, where
    /// the GL context resides.
    pub fn upload_to_gpu(&mut self) -> AssetResult<&T::AsGpu> {
        assert_state!(self, ResourceState::InMemory);

        if !janus::gl::has_gl_init() {
            return Err(AssetError::NoGlContext);
        }

        let raw_resource = self.raw_resource.as_ref().unwrap();
        let gpu_resource = raw_resource.upload_to_gpu()?;

        self.gpu_resource = Some(gpu_resource);
        self.state = ResourceState::Processed;

        Ok(self.gpu_resource.as_ref().unwrap())
    }

    // /// Acquire ownership of the GPU resource
    // pub fn take_from_gpu(&mut self) -> AssetResult<T::AsGpu> {
    //     assert_state!(self, ResourceState::InMemory);

    //     if !janus::gl::has_gl_init() {
    //         return Err(AssetError::NoGlContext);
    //     }
    // }
}

pub trait Import {
    fn from_file<P: AsRef<Path> + Debug>(path: P) -> AssetResult<Self>
    where
        Self: Sized,
    {
        let bytes = std::fs::read(&path).map_err(|io_err| AssetError::FileIoError(io_err))?;
        Self::from_memory(&bytes)
    }

    fn from_memory(bytes: &[u8]) -> AssetResult<Self>
    where
        Self: Sized;
}

pub trait IntoGpu {
    type AsGpu;

    fn upload_to_gpu(&self) -> AssetResult<Self::AsGpu>;

    fn into_gpu(self) -> AssetResult<Self::AsGpu>
    where
        Self: Sized,
    {
        Self::upload_to_gpu(&self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(AssetId);

impl Into<StringHash> for TextureId {
    fn into(self) -> StringHash {
        self.0.0
    }
}

impl AsView for Texture {
    type View = TextureView;

    fn as_view(&self) -> Self::View {
        self.view()
    }
}

#[derive(Clone, Debug)]
pub struct RawTexture(DynamicImage);

impl From<DynamicImage> for RawTexture {
    fn from(value: DynamicImage) -> Self {
        Self::new(value)
    }
}

impl RawTexture {
    pub const fn new(image: DynamicImage) -> Self {
        Self(image)
    }

    pub const fn image(&self) -> &DynamicImage {
        &self.0
    }
}

impl Import for RawTexture {
    fn from_memory(bytes: &[u8]) -> Result<RawTexture, AssetError> {
        let image = image::load_from_memory(bytes)
            .map_err(|img_err| AssetError::FileImageLoadError(img_err))?;

        Ok(Self(image))
    }
}

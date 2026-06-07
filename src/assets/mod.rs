use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

use image::DynamicImage;
use janus::{
    StringHash,
    texture::{Texture, TextureError, TextureView},
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
pub struct AssetRegistry<T: Import + Upload> {
    assets: HashMap<StringHash, Handle<T>, janus::StringHasher>,
}

impl<T: Import + Upload> AssetRegistry<T> {
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

    pub fn register<P: AsRef<Path>>(&mut self, id: impl Into<StringHash>, path: P) -> &Handle<T> {
        let id = id.into();
        let handle = Handle::new(path.as_ref().to_path_buf());
        self.assets.insert(id, handle);
        self.assets.get(&id).unwrap()
    }
}

pub trait AsView {
    type View;

    fn as_view(&self) -> Self::View;
}

// impl<T: AsView> AssetRegistry<T> {
//     pub fn get_view(&self, id: impl Into<StringHash>) -> Option<T::View> {
//         self.assets.get(&id.into()).map(AsView::as_view)
//     }
// }

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

    #[error("failed to upload texture onto gpu: unsupported image format for texture")]
    TextureUnsupportedImageFormat,

    #[error("failed to upload texture onto gpu: unknown texture upload error")]
    TextureUnknownUploadError,
}

pub type AssetResult<T> = Result<T, AssetError>;

#[derive(Debug)]
pub struct Handle<T>
where
    T: Import + Upload,
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
    T: Import + Upload,
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

    /// Invalidates the asset's memory state to [`ResourceState::Unloaded`].
    ///
    /// This allows the asset's resource to be reloaded or replaced with
    /// [`Self::load_to_memory`] and, if needed, [`Self::upload_to_gpu`] later.
    ///
    /// This can be useful if the data has changed on the disk, and needs to be
    /// updated.
    pub fn invalidate_disk(&mut self) {
        self.state = ResourceState::Unloaded;
    }

    /// Invalidates the asset's gpu state to [`ResourceState::InMemory`].
    ///
    /// This allows the asset's resource to be reloaded or replaced with
    /// [`Self::upload_to_gpu`] later.
    /// This can be useful if the data has changed in memory, and needs to be
    /// updated on the gpu.
    pub fn invalidate_gpu(&mut self) {
        self.state = ResourceState::InMemory;
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
    /// If the operation is successful, a borrow to the `T` raw resource that
    /// was just loaded is returned.
    ///
    /// [`AssetError::InvalidState`]: InvalidState
    /// [`AssetError::FileNotFound`]: FileNotFound
    /// [`AssetError::FileIoError`]: FileIoError
    /// [`AssetError::FileImageLoadError`]: FileImageLoadError
    pub fn load_to_memory(&mut self) -> AssetResult<&T> {
        if self.raw_resource.is_some() {
            assert_state!(self, ResourceState::Unloaded);
        }

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
    ///
    /// # Returns
    /// Any error as according to [`T::Upload::upload_to_gpu`], or
    /// [`AssetError::NoGlContext`] is returned if the GL context is
    /// unavailable on this thread.
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

    /// Destroy the gpu resource.
    ///
    /// This operation must be called on the graphics/windowing thread, where
    /// the GL context resides.
    ///
    /// This assumed the `Drop` implementation for the `T::AsGpu` type handles
    /// freeing and destroying the resource.
    ///
    /// If the resource is still loaded in memory, the asset's state is
    /// reversed to [`ResourceState::InMemory`], so it can be uploaded to the
    /// gpu again immediately after with [`Self::upload_to_gpu`].
    /// Otherwise, the asset's state is invalidated to
    /// [`ResourceState::Unloaded`].
    ///
    /// # Returns
    /// [`AssetError::NoGlContext`] is returned if the GL context is
    /// unavailable on this thread.
    pub fn free_from_gpu(&mut self) -> AssetResult<()> {
        assert_state!(self, ResourceState::InMemory);

        if !janus::gl::has_gl_init() {
            return Err(AssetError::NoGlContext);
        }

        let gpu_resource = self.gpu_resource.take();
        drop(gpu_resource);

        if self.raw_resource.is_some() {
            self.state = ResourceState::InMemory;
        } else {
            self.state = ResourceState::Unloaded;
        }

        Ok(())
    }

    /// Take ownership of the asset's resource in memory.
    ///
    /// This operation will invalidate the resource's state to
    /// [`ResourceState::Unloaded`] if it is not loaded on the gpu.
    pub fn take_from_memory(&mut self) -> AssetResult<T> {
        if self.raw_resource.is_none() {
            assert_state!(self, ResourceState::InMemory);
        }

        let raw_resource = self.raw_resource.take().unwrap();
        if self.state == ResourceState::InMemory {
            self.state = ResourceState::Unloaded;
        }

        Ok(raw_resource)
    }

    /// Free the asset's resource from memory.
    ///
    /// This operation takes ownership of the resource with
    /// [`Self::take_from_memory`], then drops the acquired resource right
    /// after.
    pub fn free_from_memory(&mut self) -> AssetResult<()> {
        let resource = self.take_from_memory()?;
        drop(resource);
        Ok(())
    }
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

pub trait Upload {
    type AsGpu: Debug;

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

impl Upload for RawTexture {
    type AsGpu = Texture;

    /// Upload raw texture data to the gpu.
    ///
    /// # Returns
    /// The [`Texture`] handle to the new gpu resource.
    ///
    /// This function will return [`AssetError::UnsupportedImageFormat`] if the
    /// image format is not supported, or
    /// [`AssetError::TextureUnknownUploadError`] for any other texture upload
    /// errors.
    ///
    fn upload_to_gpu(&self) -> AssetResult<Self::AsGpu> {
        let texture = Texture::from_image(&self.0).map_err(|tex_err| match tex_err {
            TextureError::UnsupportedFormat => AssetError::TextureUnsupportedImageFormat,
            TextureError::ImageLoadError(_) => unreachable!("image is already loaded"),
            _ => AssetError::TextureUnknownUploadError,
        })?;

        Ok(texture)
    }
}

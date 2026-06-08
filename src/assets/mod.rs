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
use tracing::{Level, event};

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
        std::sync::LazyLock::new(|| {
            let hashed = $crate::lazy_hash_str!($se);
            $crate::assets::AssetId(*hashed)
        })
    };
    ($sl:literal) => {
        std::sync::LazyLock::new(|| {
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

    pub fn count(&self) -> usize {
        self.assets.len()
    }

    pub fn register<P: AsRef<Path> + std::fmt::Display>(
        &mut self,
        id: impl Into<StringHash>,
        path: P,
    ) -> &Handle<T> {
        let id = id.into();
        let handle = Handle::new(path.as_ref().to_path_buf());
        self.assets.insert(id, handle);
        event!(Level::INFO, "Register asset hash_id {id}, file path {path}");
        self.assets.get(&id).unwrap()
    }

    pub fn unregister(&mut self, id: impl Into<StringHash>) -> Option<Handle<T>> {
        let id = id.into();
        event!(Level::INFO, "Unregister asset hash_id {id}");
        self.assets.remove(&id)
    }

    pub fn get(&self, id: impl Into<StringHash>) -> Option<&Handle<T>> {
        self.assets.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<StringHash>) -> Option<&mut Handle<T>> {
        self.assets.get_mut(&id.into())
    }

    pub fn contains(&self, id: impl Into<StringHash>) -> bool {
        self.assets.contains_key(&id.into())
    }
}

impl<T: Import + Upload> AssetRegistry<T>
where
    <T as Upload>::AsGpu: AsView,
{
    pub fn get_gpu_view(&self, id: impl Into<StringHash>) -> Option<<T::AsGpu as AsView>::View> {
        self.assets.get(&id.into()).map(Handle::gpu_view).flatten()
    }
}

pub trait AsView {
    type View;

    fn as_view(&self) -> Self::View;
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("resource is not present in video memory")]
    NotProcessed,

    #[error("failed to process resource for gpu: already processed")]
    AlreadyProcessed,

    #[error("resource is not present in memory")]
    NotInMemory,

    #[error("failed to load resource: already loaded in memory")]
    AlreadyInMemory,

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
    source: PathBuf,
    raw_resource: Option<T>,
    gpu_resource: Option<T::AsGpu>,
}

impl<T> Handle<T>
where
    T: Import + Upload,
    <T as Upload>::AsGpu: AsView,
{
    pub fn gpu_view(&self) -> Option<<T::AsGpu as AsView>::View> {
        self.gpu_resource
            .as_ref()
            .map(<T::AsGpu as AsView>::as_view)
    }
}

impl<T> Handle<T>
where
    T: Import + Upload,
{
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            source: path.as_ref().to_path_buf(),
            raw_resource: None,
            gpu_resource: None,
        }
    }

    pub fn is_unloaded(&self) -> bool {
        self.raw_resource.is_none() && self.gpu_resource.is_none()
    }

    pub const fn is_in_memory(&self) -> bool {
        self.raw_resource.is_some()
    }

    pub const fn is_in_gpu(&self) -> bool {
        self.gpu_resource.is_some()
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
    /// If the operation is successful, a borrow to the `T` raw resource that
    /// was just loaded is returned.
    ///
    /// [`AssetError::InvalidState`]: InvalidState
    /// [`AssetError::FileNotFound`]: FileNotFound
    /// [`AssetError::FileIoError`]: FileIoError
    /// [`AssetError::FileImageLoadError`]: FileImageLoadError
    pub fn load_to_memory(&mut self) -> AssetResult<&T> {
        if self.raw_resource.is_some() {
            return Err(AssetError::AlreadyInMemory);
        }

        let path = &self.source;
        if !path.is_file() {
            return Err(AssetError::FileNotFound(path.to_path_buf()));
        }

        let loaded = T::from_file(path)?;
        self.raw_resource = Some(loaded);
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
        if self.raw_resource.is_none() {
            return Err(AssetError::NotInMemory);
        }

        if !janus::gl::has_gl_init() {
            return Err(AssetError::NoGlContext);
        }

        let raw_resource = self.raw_resource.as_ref().unwrap();
        let gpu_resource = raw_resource.upload_to_gpu()?;
        self.gpu_resource = Some(gpu_resource);
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
        if self.gpu_resource.is_none() {
            return Err(AssetError::NotProcessed);
        }
        if !janus::gl::has_gl_init() {
            return Err(AssetError::NoGlContext);
        }

        let gpu_resource = self.gpu_resource.take();
        drop(gpu_resource);
        Ok(())
    }

    /// Take ownership of the asset's resource in memory.
    ///
    /// This operation will invalidate the resource's state to
    /// [`ResourceState::Unloaded`] if it is not loaded on the gpu.
    pub fn take_from_memory(&mut self) -> AssetResult<T> {
        if self.raw_resource.is_none() {
            return Err(AssetError::NotInMemory);
        }

        let raw_resource = self.raw_resource.take().unwrap();
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

#[macro_export]
macro_rules! asset_manager {
    (struct $asset:ty {
        $($name:ident: $path:expr;)*
    }) => {
        paste::paste! {
            $(
                const [< $asset:upper _ $name:upper >]: std::sync::LazyLock<$crate::assets::AssetId> = $crate::hashet!(stringify!($name));
                const [< $asset:upper _ $name:upper _PATH >]: &'static str = $path;
            )*

            const [< $asset:upper _MANAGER >]: std::cell::LazyCell<$crate::assets::AssetRegistry<$asset>> = std::cell::LazyCell::new(|| {
                let mut asset_manager = $crate::assets::AssetRegistry::new();

                {
                    $(
                        let hash_id = *[< $asset:upper _ $name:upper >];
                        let path = [< $asset:upper _ $name:upper _PATH >];
                        asset_manager.register(hash_id, path);
                    )*
                }

                asset_manager
            });
        }
    };
}

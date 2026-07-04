use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

use image::DynamicImage;
use janus::{
    GpuResource, StringHash,
    texture::{ImageFormat, ImageType, Texture, TextureError, TextureKey, TextureView},
};
use serde::{Deserialize, Serialize};
use tracing::{Level, event};

use crate::assets::pipe::{AssetMessage, AssetMessageRequest, AssetSyncMessage};

pub mod pipe;
pub mod strings;

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    (
        $($pv:vis const $n:ident = $v:expr;)+
    ) => {
        $(
            $pv const $n: std::sync::LazyLock<$crate::assets::AssetId> = $crate::hashet!($v);
        )+
    };
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

#[derive(Debug)]
pub struct AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    assets: HashMap<StringHash, Handle<T, M>, janus::StringHasher>,
    pipe_tx: crossbeam::channel::Sender<AssetMessage>,
    pipe_rx: crossbeam::channel::Receiver<AssetMessage>,
    sync_pipe_tx: Option<crossbeam::channel::Sender<AssetSyncMessage<M>>>,
}
impl<T, M> Default for AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    fn default() -> Self {
        let (pipe_tx, pipe_rx) = crossbeam::channel::unbounded();
        Self {
            assets: Default::default(),
            pipe_tx,
            pipe_rx,
            sync_pipe_tx: None,
        }
    }
}
impl<T, M> AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    pub fn new() -> Self {
        Self {
            assets: HashMap::with_hasher(janus::StringHasher::default()),
            ..Default::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            assets: HashMap::with_capacity_and_hasher(capacity, janus::StringHasher::default()),
            ..Default::default()
        }
    }

    pub fn create_metadata_registry(&mut self) -> AssetMetadataRegistry<M> {
        let mut registry = AssetMetadataRegistry::new();
        self.assets.iter().for_each(|(&id, asset)| {
            let meta = asset.metadata();
            registry.mapping.insert(id, meta);
        });
        self.sync_pipe_tx = Some(registry.sync_pipe());
        registry
    }

    pub fn count(&self) -> usize {
        self.assets.len()
    }

    pub fn register<P: AsRef<Path>>(
        &mut self,
        id: impl Into<StringHash>,
        path: P,
    ) -> &Handle<T, M> {
        let id = id.into();
        let handle = Handle::new(id, path.as_ref().to_path_buf(), &self);
        self.assets.insert(id, handle);

        if let Some(sync_pipe) = &self.sync_pipe_tx {
            sync_pipe
                .send(AssetSyncMessage::Register {
                    id,
                    data: Default::default(),
                })
                .unwrap();
        }

        event!(
            Level::INFO,
            "Register asset hash_id {id}, file path {}",
            path.as_ref().display()
        );
        self.assets.get(&id).unwrap()
    }

    pub fn unregister(&mut self, id: impl Into<StringHash>) -> Option<Handle<T, M>> {
        let id = id.into();

        if let Some(sync_pipe) = &self.sync_pipe_tx {
            sync_pipe.send(AssetSyncMessage::Forget { id }).unwrap();
        }

        event!(Level::INFO, "Unregister asset hash_id {id}");
        self.assets.remove(&id)
    }

    pub fn get(&self, id: impl Into<StringHash>) -> Option<&Handle<T, M>> {
        self.assets.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<StringHash>) -> Option<&mut Handle<T, M>> {
        self.assets.get_mut(&id.into())
    }

    pub fn contains(&self, id: impl Into<StringHash>) -> bool {
        self.assets.contains_key(&id.into())
    }
}
impl<T, M> AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    M: Default + Clone + Copy,
    <T as Upload>::AsGpu: HasMetadata<M>,
    <T as Upload>::AsGpu: AsView,
{
    pub fn get_gpu_view(&self, id: impl Into<StringHash>) -> Option<<T::AsGpu as AsView>::View> {
        self.assets.get(&id.into()).map(Handle::gpu_view).flatten()
    }
}
impl<T, M> AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M> + AsView,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    pub fn get_resource_view(&self, id: impl Into<StringHash>) -> Option<<T as AsView>::View> {
        self.assets
            .get(&id.into())
            .map(Handle::resource_view)
            .flatten()
    }
}

#[derive(Debug, Clone)]
pub struct AssetMetadataRegistry<M: Default + Clone + Copy> {
    mapping: janus::StringMap<M>,
    sync_rx: crossbeam::channel::Receiver<AssetSyncMessage<M>>,
    sync_tx: crossbeam::channel::Sender<AssetSyncMessage<M>>,
}
impl<M: Default + Clone + Copy> AssetMetadataRegistry<M> {
    pub fn new() -> Self {
        let (sync_tx, sync_rx) = crossbeam::channel::unbounded();
        Self {
            mapping: janus::StringMap::default(),
            sync_rx,
            sync_tx,
        }
    }

    pub fn sync_pipe(&self) -> crossbeam::channel::Sender<AssetSyncMessage<M>> {
        self.sync_tx.clone()
    }

    pub fn pipe_sync_commands(&mut self) {
        while let Ok(command) = self.sync_rx.try_recv() {
            match command {
                AssetSyncMessage::Register { id, data } => {
                    self.mapping.insert(id, data);
                }
                AssetSyncMessage::Update { id, data } => {
                    if let Some(meta) = self.mapping.get_mut(&id) {
                        *meta = data;
                    }
                }
                AssetSyncMessage::Forget { id } => {
                    self.mapping.remove(&id);
                }
            }
        }
    }

    pub fn get(&self, id: impl Into<StringHash>) -> Option<M> {
        self.mapping.get(&id.into()).copied()
    }
}

pub trait AsView {
    type View;

    fn as_view(&self) -> Self::View;
}

pub trait HasMetadata<T: Default + Clone + Copy> {
    fn metadata(&self) -> T {
        let mut meta = T::default();
        self.make_metadata(&mut meta);
        meta
    }

    fn make_metadata(&self, metadata: &mut T);
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("requested asset {0} not found in registry")]
    AssetNotFound(StringHash),

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
impl PartialEq for AssetError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileNotFound(l0), Self::FileNotFound(r0)) => l0 == r0,
            (Self::FileIoError(l0), Self::FileIoError(r0)) => l0.kind() == r0.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for AssetError {}

pub type AssetResult<T> = Result<T, AssetError>;

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct Handle<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    id: StringHash,
    source: PathBuf,
    #[serde(skip)]
    raw_resource: Option<T>,
    #[serde(skip)]
    gpu_resource: Option<T::AsGpu>,
    #[serde(skip)]
    root_pipe: crossbeam::channel::Sender<AssetMessage>,
    #[serde(skip)]
    _marker_meta: std::marker::PhantomData<M>,
}
impl<T, M> Handle<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    M: Default + Clone + Copy,
    <T as Upload>::AsGpu: AsView + HasMetadata<M>,
{
    pub fn gpu_view(&self) -> Option<<T::AsGpu as AsView>::View> {
        self.gpu_resource
            .as_ref()
            .map(<T::AsGpu as AsView>::as_view)
    }
}
impl<T, M> Handle<T, M>
where
    T: Import + Upload + AsView + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    pub fn resource_view(&self) -> Option<<T as AsView>::View> {
        self.raw_resource.as_ref().map(<T as AsView>::as_view)
    }
}
impl<T, M> HasMetadata<M> for Handle<T, M>
where
    T: Import + Upload,
    M: Default + Clone + Copy,
    T: HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
{
    fn make_metadata(&self, metadata: &mut M) {
        if let Some(raw) = &self.raw_resource {
            raw.make_metadata(metadata);
        }
        if let Some(gpu) = &self.gpu_resource {
            gpu.make_metadata(metadata);
        }
    }
}
impl<T, M> Handle<T, M>
where
    M: Default + Clone + Copy,
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
{
    pub fn from_resource(id: StringHash, resource: T, registry: &AssetRegistry<T, M>) -> Self {
        Self {
            id,
            source: PathBuf::new(),
            raw_resource: Some(resource),
            gpu_resource: None,
            root_pipe: registry.command_pipe(),
            _marker_meta: std::marker::PhantomData,
        }
    }

    pub fn from_gpu_resource(
        id: StringHash,
        resource: T::AsGpu,
        registry: &AssetRegistry<T, M>,
    ) -> Self {
        Self {
            id,
            source: PathBuf::new(),
            raw_resource: None,
            gpu_resource: Some(resource),
            root_pipe: registry.command_pipe(),
            _marker_meta: std::marker::PhantomData,
        }
    }

    pub fn new<P: AsRef<Path>>(id: StringHash, path: P, registry: &AssetRegistry<T, M>) -> Self {
        Self {
            id,
            source: path.as_ref().to_path_buf(),
            raw_resource: None,
            gpu_resource: None,
            root_pipe: registry.command_pipe(),
            _marker_meta: std::marker::PhantomData,
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

        self.root_pipe
            .send(AssetMessage::Success {
                reference_id: self.id,
                operation: AssetMessageRequest::LoadToMemory,
            })
            .unwrap();

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

        self.root_pipe
            .send(AssetMessage::Success {
                reference_id: self.id,
                operation: AssetMessageRequest::LoadToGpu,
            })
            .unwrap();

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

        self.root_pipe
            .send(AssetMessage::Success {
                reference_id: self.id,
                operation: AssetMessageRequest::UnloadFromGpu,
            })
            .unwrap();

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

        self.root_pipe
            .send(AssetMessage::Success {
                reference_id: self.id,
                operation: AssetMessageRequest::UnloadFromMemory,
            })
            .unwrap();

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

#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TextureId(pub AssetId);
impl From<AssetId> for TextureId {
    fn from(value: AssetId) -> Self {
        Self(value)
    }
}
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
impl HasMetadata<TextureMetadata> for RawTexture {
    fn make_metadata(&self, metadata: &mut TextureMetadata) {
        let w = self.0.width();
        let h = self.0.height();
        metadata.size = Some((w, h));
    }
}
impl HasMetadata<TextureMetadata> for Texture {
    fn make_metadata(&self, metadata: &mut TextureMetadata) {
        let image_format = self.metadata.format();
        let pixel_format = self.metadata.pixel();
        let gl_object = TextureKey(self.resource_id());
        metadata.image_format = Some(image_format);
        metadata.pixel_format = Some(pixel_format);
        metadata.gl_object = Some(gl_object);
    }
}
impl HasMetadata<TextureMetadata> for TextureView {
    fn make_metadata(&self, metadata: &mut TextureMetadata) {
        let image_format = self.metadata().image_format;
        let pixel_format = self.metadata().pixel_format;
        let gl_object = TextureKey(self.resource_id());
        metadata.image_format = image_format;
        metadata.pixel_format = pixel_format;
        metadata.gl_object = Some(gl_object);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TextureMetadata {
    pub size: Option<(u32, u32)>,
    pub image_format: Option<ImageFormat>,
    pub pixel_format: Option<ImageType>,
    pub gl_object: Option<TextureKey>,
}

#[macro_export]
macro_rules! asset_registry {
    (struct $asset:ty: $meta:ty {
        $($name:ident: $path:expr;)*
    }) => {
        paste::paste! {
            $(
                const [< $asset:upper _ $name:upper >]: std::sync::LazyLock<$crate::assets::AssetId> = $crate::hashet!(stringify!($name));
                const [< $asset:upper _ $name:upper _PATH >]: &'static str = $path;
            )*

            pub struct [< $asset RegistryBuilder >];

            impl [< $asset RegistryBuilder >] {
                pub fn build() -> $crate::assets::AssetRegistry<$asset, $meta> {
                    let mut asset_manager = $crate::assets::AssetRegistry::new();

                    {
                        $(
                            let hash_id = *[< $asset:upper _ $name:upper >];
                            let path = [< $asset:upper _ $name:upper _PATH >];
                            asset_manager.register(hash_id, path);
                        )*
                    }

                    asset_manager
                }
            }
        }
    };
}

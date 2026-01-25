use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use crate::render::data::{StorageSection, SyncBarrier, SyncState};

/// The shared storage boundary.
///
/// This represents the common shared state between the [`consumer cross`] and
/// the [`producer cross`].
///
/// The [`Boundary`] handles common synchronisation caching and keeps track of
/// the current working section of the buffer.
///
/// It also contains the actual `Storage`, such as [`RenderStorage`].
///
/// [`RenderStorage`]: crate::render::data::RenderStorage
pub struct Boundary<Storage> {
    storage: Storage,
    working_section: AtomicU8,
    sync_cache: SyncState,
}

impl<Storage> Boundary<Storage> {
    pub fn new(storage: Storage) -> Self {
        let working_section = AtomicU8::new(StorageSection::Spare as u8);
        let sync_cache = SyncState::new();
        Self {
            storage,
            working_section,
            sync_cache,
        }
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn current_section(&self) -> StorageSection {
        StorageSection::from_byte(self.working_section.load(Ordering::Acquire))
    }

    pub fn advance_section(&self) {
        self.working_section
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |byte| {
                Some(StorageSection::from_byte(byte).next() as u8)
            })
            .expect("function never returns None");
    }

    pub fn sync_cache(&self) -> &SyncState {
        &self.sync_cache
    }

    fn sync(&self, barrier: &mut SyncBarrier) {
        barrier.fetch(&self.sync_cache);
    }
}

/// The consumer is the "reader" over the shared storage.
///
/// The consumer works directly on the current buffer section.
///
/// The consumer does not check for locks, as it is the part of the boundary
/// that commands the locks.
///
pub struct Consumer;

/// The producer is the "writer" over the shared storage.
///
/// The producer works on the *next* section of the buffer. After it is done,
/// it advances the current buffer to the next section.
///
/// It will only operate if the section of the buffer it is working on is not
/// currently under a lock. Otherwise, the operation safely aborts.
pub struct Producer;

/// Operator over a [`shared storage boundary`](Boundary).
///
/// This can either be:
/// * A [`Consumer`], a "reader" over the shared storage
/// * A [`Producer`], a "writer" over the shared storage
///
/// See the documentation for the respective types for more information.
pub struct Cross<Role, Storage> {
    boundary: Arc<Boundary<Storage>>,
    _role: std::marker::PhantomData<Role>,
    _storage: std::marker::PhantomData<Storage>,
}

impl<Role, Storage> Cross<Role, Storage> {
    pub fn new(shared_boundary: Arc<Boundary<Storage>>) -> Self {
        Self {
            boundary: shared_boundary,
            _role: std::marker::PhantomData,
            _storage: std::marker::PhantomData,
        }
    }
}

impl<Storage> Cross<Consumer, Storage> {
    /// Let the [`Consumer`] cross the [`Boundary`], as a "read" operation.
    ///
    /// This will operate under the current buffer section.
    ///
    /// The [`Boundary`]'s synchronisation cache fetches the current state from
    /// `barrier` before and after the `op` operation executes over the shared
    /// storage.
    ///
    /// This means that the GPU fence synchronisation of `barrier` must be
    /// handled by the caller.
    pub fn cross<F>(&self, barrier: &mut SyncBarrier, op: F)
    where
        F: Fn(StorageSection, &Storage),
    {
        let section = self.boundary.current_section();
        self.boundary.sync(barrier);
        op(section, self.boundary.storage());
        self.boundary.sync(barrier);
    }
}

impl<Storage> Cross<Producer, Storage> {
    /// Let the [`Producer`] cross the [`Boundary`], as a "write" operation.
    ///
    /// This will operate under the *next* buffer section.
    ///
    /// The `op` operation will only be executed if the lock for the next
    /// buffer section is free. Otherwise, the operation safely aborts.
    ///
    /// After the operation is executed (no lock was present on the section),
    /// the current tracked section of the [`Boundary`] is advanced to the
    /// next section (the one the CPU has just finished writing to).
    pub fn cross<F>(&self, op: F)
    where
        F: Fn(StorageSection, &Storage),
    {
        let section = self.boundary.current_section().next();
        if !self.boundary.sync_cache().has_lock(section) {
            op(section, self.boundary.storage());
            self.boundary.advance_section();
        }
    }
}

/// Create a cross-boundary storage synchroniser.
///
/// The function takes a `storage`, such as [`RenderStorage`], and will yield
/// two [`Cross`] operators: the [`Producer`] and the [`Consumer`].
///
/// The [`Producer`] and the [`Consumer`] operate on the same shared data of
/// the given `storage` and synchronise thanks to [`Boundary`].
///
/// They operate differently: the [`Producer`] is intended as an outbound
/// operation, while the [`Consumer`] is intended as an inbound operation.
///
/// See the documentation of each respective type for more information.
pub fn create<Storage>(storage: Storage) -> (Cross<Producer, Storage>, Cross<Consumer, Storage>) {
    let boundary = Arc::new(Boundary::new(storage));
    let producer = Cross::new(Arc::clone(&boundary));
    let consumer = Cross::new(Arc::clone(&boundary));
    (producer, consumer)
}

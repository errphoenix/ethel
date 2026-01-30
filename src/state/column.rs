use std::ops::{Deref, DerefMut};

/// A wrapper for an entry of a [`Column`] over the `T` type.
///
/// Other than the inner value of `T`, this also contains the owning indirect
/// index that points to this entry in its [`Column`].
///
/// The index is only 4 bytes, this means that for optimal cache-line
/// utilisation this must be taken into account.
/// On most systems, a cache-line is 64 bytes, thus the size of `T` should be
/// up to `60` bytes.
///
/// For a 64 bytes cache-line the optimal size is a factor of `64`:
/// * `8` bytes, as in: `4` for `T` + `4`.
/// * `16` bytes, as in: `12` for `T` + `4`.
/// * `32` bytes, as in: `28` for `T` + `4`.
/// * `64` bytes, as in: `60` for `T` + `4`.
///
/// Sizes `4`, `2`, and `1` are omitted for obvious reasons.
#[derive(Clone, Debug, Default)]
pub struct Entry<T> {
    owner: u32,
    inner: T,
}

impl<T> Entry<T> {
    pub fn new(owner: u32, value: T) -> Self {
        Self {
            owner,
            inner: value,
        }
    }

    /// Get the indirect index that points to this entry in its original
    /// [`Column`].
    ///
    /// The owning indirect index provided by the entry is the same indirect
    /// index that any external entity or system would use to refer to this
    /// entry.
    ///
    /// As this is a stable index, it can safely be used across entites and
    /// systems to track data without copying or reference counting.
    pub fn owner(&self) -> u32 {
        self.owner
    }

    pub fn inner_value(&self) -> &T {
        &self.inner
    }

    pub fn inner_value_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[derive(Debug, Default)]
pub struct Column<T: Default> {
    /// These indices are guaranteed to be consistent and are never moved
    /// around to maintain cache locality.
    ///
    /// Each index refers to an index into the `contiguous` data vector.
    ///
    /// Often referred to as "indirect indices".
    indices: Vec<usize>,

    /// The "real" collection. This is contiguous, optimised for cache
    /// locality.
    ///
    /// Each element is a [`Entry`] which, other than the value, also contains
    /// the index of the slot that points to the element.
    contiguous: Vec<Entry<T>>,

    /// Keeps track of free slots of the indirect `indices`.
    free: Vec<usize>,
}

impl<T: Default> Column<T> {
    /// Create a blank new Column with a size of `1`.
    ///
    /// The only element present is the degenerate element at index `0`.
    pub fn new() -> Self {
        Self {
            indices: vec![0],
            contiguous: vec![Entry::default()],
            ..Default::default()
        }
    }

    /// Creata a blank new column with the given `capacity`.
    ///
    /// All elements are initialised with their [`Default`] implementation.
    /// This includes the degenerate element at index `0`.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut stable_indices = Vec::with_capacity(capacity);
        let mut contiguous = Vec::with_capacity(capacity);

        stable_indices.push(0);
        contiguous.push(Entry::default());

        Self {
            indices: stable_indices,
            contiguous,
            ..Default::default()
        }
    }

    /// Mark the indexing slot at `index` as free.
    ///
    /// The `index` must be a stable indirect index.
    ///
    /// # Panics
    /// * If `index` is out of bounds
    /// * If `index == 0`, since that is a reserved index
    pub fn free(&mut self, index: usize) {
        if index == 0 {
            panic!("slot 0 is reserved");
        }

        let slot = self.indices[index];
        if slot == 0 {
            return;
        }
        self.indices[index] = 0;

        if let Some(owner_last) = self.contiguous.last().map(Entry::owner) {
            self.indices[owner_last as usize] = slot;
        }

        self.contiguous.swap_remove(slot);
        self.free.push(index);
    }

    fn next_slot_index(&mut self) -> usize {
        if let Some(free) = self.free.pop() {
            free
        } else {
            let i = self.indices.len();
            // uninitialised index pushed solely to ensure that an available
            // slot exists when requested, it is not tracked.
            // the stability of this data structure depends entirely on
            // replacing this dummy value with a real one before other
            // operations and avoiding "forgetting" this UNTRACKED empty slot.
            // this is done properly by Column::put.
            self.indices.push(0);
            i
        }
    }

    /// Add a `value` to the Column.
    ///
    /// This will automatically handle getting a valid slot for the inserted
    /// value:
    /// * If there is any freed slot that was previously occupied by a value
    ///   that has since been [`free'd`](Column::free), that slot will be
    ///   occupied. If there are multiple slots, no particular slot is
    ///   prioritised.
    /// * Otherwise, `value` is appended at the end of the Column. This may
    ///   cause it to grow and reallocate if the current capacity is not
    ///   sufficient.
    ///
    /// # Returns
    /// Returns the indirect index of the newly inserted [`Entry`].
    pub fn put(&mut self, value: T) -> usize {
        let index = self.next_slot_index();
        let slot = self.contiguous.len();
        self.indices[index] = slot;
        self.contiguous.push(Entry::new(index as u32, value));
        index
    }

    pub fn get_indirect(&self, index: usize) -> &T {
        let slot = self.indices[index];
        &self.contiguous[slot].inner
    }

    pub fn get_direct(&self, direct_index: usize) -> &T {
        &self.contiguous[direct_index].inner
    }

    pub fn get_indirect_mut(&mut self, index: usize) -> &mut T {
        let slot = self.indices[index];
        &mut self.contiguous[slot].inner
    }

    pub fn get_direct_mut(&mut self, direct_index: usize) -> &mut T {
        &mut self.contiguous[direct_index].inner
    }

    /// Get an immutable iterator to the inner contiguous data.
    ///
    /// This skips the degenerate element at index 0 and maps each [`Entry`] to
    /// its real inner value.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.contiguous.iter().skip(1).map(Entry::inner_value)
    }

    /// Get a mutable iterator to the inner contiguous data.
    ///
    /// This skips the degenerate element at index 0 and maps each [`Entry`] to
    /// its real inner value.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.contiguous
            .iter_mut()
            .skip(1)
            .map(Entry::inner_value_mut)
    }

    pub fn indirect(&self) -> &[usize] {
        &self.indices
    }

    /// Get an immutable slice to the inner contiguous data.
    ///
    /// Each [`Entry`] in the returned slice also contains the slot (or
    /// component id) that an external object would use to refer to this
    /// entry.
    ///
    /// Note that this also contains the degenerate element at index 0, which
    /// you likely want to skip.
    pub fn contiguous(&self) -> &[Entry<T>] {
        &self.contiguous
    }
}

impl<T: Default> IntoIterator for Column<T> {
    type Item = Entry<T>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.contiguous.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct StagingColumn<Lo: Default, Re: Default> {
    inner: Column<Lo>,
    stage: Vec<Re>,
}

impl<T, S> StagingColumn<T, S>
where
    T: Default,
    S: Default + From<T>,
{
    pub fn new() -> Self {
        Self {
            inner: Column::new(),
            stage: vec![S::default()],
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut stage = Vec::with_capacity(capacity);
        stage.push(S::default());

        Self {
            inner: Column::with_capacity(capacity),
            stage,
        }
    }

    pub fn pod(&self) -> &[S] {
        &self.stage
    }
}

impl<T, S> StagingColumn<T, S>
where
    T: Default + Clone + Copy,
    S: Default + From<T>,
{
    pub fn sync_stage(&mut self) {
        self.inner
            .iter()
            .zip(&mut self.stage)
            .for_each(|(inner, stage)| {
                *stage = S::from(*inner);
            });
    }
}

impl StagingColumn<glam::Vec3, glam::Vec4> {
    pub fn sync_stage_shuffle_vector(&mut self) {
        self.inner
            .iter()
            .zip(&mut self.stage)
            .for_each(|(inner, stage)| *stage = glam::Vec4::new(inner.x, inner.y, inner.z, 1.0));
    }
}

impl<T, S> Deref for StagingColumn<T, S>
where
    T: Default,
    S: Default + From<T>,
{
    type Target = Column<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, S> DerefMut for StagingColumn<T, S>
where
    T: Default,
    S: Default + From<T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

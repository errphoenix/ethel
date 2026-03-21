pub mod column;
pub mod hash;
pub mod table;

pub use column::{ArrayColumn, IndexArrayColumn, ParallelIndexArrayColumn};
pub use table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct IndirectIndex(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectIndex(pub(crate) u32);

impl IndirectIndex {
    pub fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub(crate) fn from_int(int: u32) -> Self {
        Self(int)
    }

    pub fn as_int(self) -> u32 {
        self.0
    }

    pub fn as_index(self) -> usize {
        self.0 as usize
    }
}

impl DirectIndex {
    pub fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub(crate) fn from_int(int: u32) -> Self {
        Self(int)
    }

    pub fn as_int(self) -> u32 {
        self.0
    }

    pub fn as_index(self) -> usize {
        self.0 as usize
    }
}

impl Into<u32> for IndirectIndex {
    fn into(self) -> u32 {
        self.as_int()
    }
}

impl Into<usize> for IndirectIndex {
    fn into(self) -> usize {
        self.as_index()
    }
}

impl Into<u32> for DirectIndex {
    fn into(self) -> u32 {
        self.as_int()
    }
}

impl Into<usize> for DirectIndex {
    fn into(self) -> usize {
        self.as_index()
    }
}

pub trait SparseSlot: Default {
    fn slots_map(&self) -> &Vec<DirectIndex>;

    fn slots_map_mut(&mut self) -> &mut Vec<DirectIndex>;

    fn free_list(&self) -> &Vec<IndirectIndex>;

    fn free_list_mut(&mut self) -> &mut Vec<IndirectIndex>;

    fn next_slot_index(&mut self) -> IndirectIndex {
        if let Some(cached_index) = self.free_list_mut().pop() {
            cached_index
        } else {
            let new_index = IndirectIndex::from_index(self.slots_map().len());
            // uninitialised index pushed solely to ensure that an available
            // slot exists when requested, it is not tracked.
            // the stability of this data structure depends entirely on
            // replacing this dummy value with a real one before other
            // operations and avoiding "forgetting" this UNTRACKED empty slot.
            // this is done properly by Column::put.
            self.slots_map_mut().push(DirectIndex::default());
            new_index
        }
    }
}

pub trait Column<T: Default>: SparseSlot + Default {
    /// The total amount of initialised slots.
    ///
    /// This includes indirect indices of degenerates (zero), as is it a sparse
    /// collection.
    fn size(&self) -> usize;

    /// The total length of the contiguous data (SoA's).
    fn len(&self) -> usize;

    /// Solve the given indirect index.
    ///
    /// The returned direct index is not a stable index and will change
    /// depending on the internal memory layout of the Column.
    #[inline]
    fn solve_indirect(&self, slot: IndirectIndex) -> Option<DirectIndex> {
        self.slots_map().get(slot.as_index()).copied()
    }

    /// Solve the given indirect index.
    ///
    /// The returned direct index is not a stable index and will change
    /// depending on the internal memory layout of the Column.
    ///
    /// # Safety
    /// Caller must ensure that the given `slot` is always a valid index within
    /// the bounds of the table.
    /// Otherwise, the function will produce undefined behaviour.
    #[inline]
    unsafe fn solve_indirect_unchecked(&self, slot: IndirectIndex) -> DirectIndex {
        // SAFETY: the caller must ensure that `slot` is always a valid index
        //         within bounds
        unsafe { *self.slots_map().get_unchecked(slot.as_index()) }
    }

    /// Mark the given indirect index as free.
    ///
    /// # Panics
    /// * If `slot` is out of bounds in the sparse index array
    /// * If `slot == 0`, since it is a reserved slot to mark degenerate
    ///   elements
    fn free(&mut self, slot: IndirectIndex);

    /// Add an element `value` to the inner SoA storage.
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
    /// Returns the indirect index of the newly inserted element.
    fn insert(&mut self, value: T) -> IndirectIndex;
}

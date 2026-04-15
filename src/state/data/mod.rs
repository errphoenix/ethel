pub mod column;
pub mod hash;
pub mod table;

pub use column::{ArrayColumn, IndexArrayColumn, ParallelIndexArrayColumn};
pub use table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct IndirectIndex {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DirectIndex {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl IndirectIndex {
    pub fn null(generation: u32) -> Self {
        Self {
            index: 0,
            generation,
        }
    }

    pub fn from_index(index: usize, generation: u32) -> Self {
        Self {
            index: index as u32,
            generation,
        }
    }

    pub fn from_int(int: u32, generation: u32) -> Self {
        Self {
            index: int,
            generation,
        }
    }

    pub fn next_generation(self) -> Self {
        Self {
            index: 0,
            generation: self.generation + 1,
        }
    }

    pub fn related_to_direct(&self, direct: &DirectIndex) -> bool {
        self.generation == direct.generation
    }

    pub fn related_to(&self, other: &IndirectIndex) -> bool {
        self.eq(other)
    }

    pub fn as_int(self) -> u32 {
        self.index
    }

    pub fn as_index(self) -> usize {
        self.index as usize
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl DirectIndex {
    pub fn null(generation: u32) -> Self {
        Self {
            index: 0,
            generation,
        }
    }

    pub fn from_index(index: usize, generation: u32) -> Self {
        Self {
            index: index as u32,
            generation,
        }
    }

    pub fn from_int(int: u32, generation: u32) -> Self {
        Self {
            index: int,
            generation,
        }
    }

    pub fn next_generation(self) -> Self {
        Self {
            index: 0,
            generation: self.generation + 1,
        }
    }

    pub fn related_to_indirect(&self, indirect: &IndirectIndex) -> bool {
        self.generation == indirect.generation
    }

    pub fn related_to(&self, other: &DirectIndex) -> bool {
        self.eq(other)
    }

    pub fn as_int(self) -> u32 {
        self.index
    }

    pub fn as_index(self) -> usize {
        self.index as usize
    }

    pub fn generation(&self) -> u32 {
        self.generation
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
            // cached index's generatin is already updated, since it was freed
            cached_index
        } else {
            // new index, gen 0
            let new_index = IndirectIndex::from_index(self.slots_map().len(), 0);

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
        if let Some(index) = self.slots_map().get(slot.as_index()).copied()
            && index.generation == slot.generation
        {
            Some(index)
        } else {
            None
        }
    }

    /// Solve the given indirect index.
    ///
    /// The returned direct index is not a stable index and will change
    /// depending on the internal memory layout of the Column.
    ///
    /// This also does not check for the indices' generations to be equal.
    ///
    /// In most cases, you'll want to stick with
    /// [`solve_indirect`](Self::solve_indirect), which maintains all security
    /// guarantees.
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

    fn free_many(&mut self, slots: &[IndirectIndex]) {
        slots.iter().for_each(|&slot| self.free(slot));
    }

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
    fn insert<V: Into<T>>(&mut self, value: V) -> IndirectIndex;
}

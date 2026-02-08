pub mod column;
pub mod table;

pub use column::{ArrayColumn, IndexArrayColumn, ParallelIndexArrayColumn};
pub use table::Table;

pub trait SparseSlot: Default {
    fn slots_map(&self) -> &Vec<u32>;

    fn slots_map_mut(&mut self) -> &mut Vec<u32>;

    fn free_list(&self) -> &Vec<u32>;

    fn free_list_mut(&mut self) -> &mut Vec<u32>;

    fn next_slot_index(&mut self) -> u32 {
        if let Some(cached_index) = self.free_list_mut().pop() {
            cached_index
        } else {
            let new_index = self.slots_map().len() as u32;
            // uninitialised index pushed solely to ensure that an available
            // slot exists when requested, it is not tracked.
            // the stability of this data structure depends entirely on
            // replacing this dummy value with a real one before other
            // operations and avoiding "forgetting" this UNTRACKED empty slot.
            // this is done properly by Column::put.
            self.slots_map_mut().push(0);
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

    /// Get the indirect index present at `slot`.
    ///
    /// The returned indirect index is not a stable index and will change
    /// depending on the internal memory layout of the Column.
    #[inline]
    fn get_indirect(&self, slot: u32) -> Option<u32> {
        self.slots_map().get(slot as usize).copied()
    }

    /// Get the indirect index present at `slot`.
    ///
    /// The returned indirect index is not a stable index and will change
    /// depending on the internal memory layout of the Column.
    ///
    /// # Panics
    /// If the given `slot` is not present in the slots map; i.e. it is out of
    /// bounds.
    #[inline]
    fn get_indirect_unchecked(&self, slot: u32) -> u32 {
        self.slots_map()[slot as usize]
    }

    /// Mark the indexing slot at `slot` as free.
    ///
    /// The `slot` must be a stable indirect index (slot).
    ///
    /// # Panics
    /// * If `slot` is out of bounds
    /// * If `slot == 0`, since it is a reserved slot to mark degenerate
    ///   elements
    fn free(&mut self, slot: u32);

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
    fn put(&mut self, value: T) -> u32;
}

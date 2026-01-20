#[derive(Clone, Debug, Default)]
pub struct Entry<T> {
    owner: usize,
    inner: T,
}

impl<T> Entry<T> {
    pub fn new(owner: usize, value: T) -> Self {
        Self {
            owner,
            inner: value,
        }
    }

    pub fn owner(&self) -> usize {
        self.owner
    }

    pub fn inner_value(&self) -> &T {
        &self.inner
    }
}

#[derive(Debug, Default)]
pub struct Column<T> {
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
    pub fn new() -> Self {
        Self {
            indices: vec![0],
            contiguous: vec![Default::default()],
            ..Default::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut stable_indices = Vec::with_capacity(capacity);
        let mut contiguous = Vec::with_capacity(capacity);

        stable_indices.push(0);
        contiguous.push(Default::default());

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
            self.indices[owner_last] = index;
        }

        self.contiguous.swap_remove(slot);
        self.free.push(index);
    }

    fn next_slot_index(&mut self) -> usize {
        if let Some(free) = self.free.pop() {
            free
        } else {
            let i = self.indices.len();
            // uninitialised index
            self.indices.push(0);
            i
        }
    }

    pub fn put(&mut self, value: T) -> usize {
        let index = self.next_slot_index();
        let slot = self.contiguous.len();
        self.indices[index] = slot;
        self.contiguous.push(Entry::new(index, value));
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

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.contiguous.iter().map(Entry::inner_value)
    }

    pub fn direct(&self) -> &Vec<Entry<T>> {
        &self.contiguous
    }
}

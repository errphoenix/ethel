use std::borrow::{Borrow, BorrowMut};

use crate::state::data::{Column, SparseSlot};

/// A wrapper for an entry of an [`IndexArrayColumn`] over the `T` type.
///
/// Other than the inner value of `T`, this also contains the owning indirect
/// index that points to this entry in its [`IndexArrayColumn`].
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
#[repr(C)]
#[derive(Clone, Debug, Default)]
pub struct Entry<T> {
    inner: T,
    owner: u32,
}

impl<T> Entry<T> {
    pub fn new(owner: u32, value: T) -> Self {
        Self {
            owner,
            inner: value,
        }
    }

    /// Get the indirect index that points to this entry in its original
    /// [`IndexArrayColumn`].
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

impl<T> Borrow<T> for Entry<T> {
    fn borrow(&self) -> &T {
        self.inner_value()
    }
}

impl<T> BorrowMut<T> for Entry<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.inner_value_mut()
    }
}

pub trait IterColumn<'iter, T, R>
where
    T: Default,
    R: Default + Borrow<T> + BorrowMut<T> + 'iter,
{
    fn contiguous(&self) -> &[R];

    fn contiguous_mut(&mut self) -> &mut [R];

    /// Get an immutable iterator to the inner contiguous data.
    ///
    /// This skips the first degenerate element at index 0.
    ///
    /// # Returns
    /// The data present in the inner contiguous collection.
    ///
    /// For [`IndexArrayColumn`], this does not return `T` but an [`Entry`] wrapping
    /// the real `T` value.
    ///
    /// See [`Entry`] for more info on managing this type and memory layout
    /// considerations.
    #[inline]
    fn iter(&'iter self) -> impl Iterator<Item = &'iter R> {
        self.contiguous().iter().skip(1)
    }

    /// Get an mutable iterator to the inner contiguous data.
    ///
    /// This skips the first degenerate element at index 0.
    ///
    /// # Returns
    /// The data present in the inner contiguous collection.
    ///
    /// For [`IndexArrayColumn`], this does not return `T` but an [`Entry`] wrapping
    /// the real `T` value.
    ///
    /// See [`Entry`] for more info on managing this type and memory layout
    /// considerations.
    #[inline]
    fn iter_mut(&'iter mut self) -> impl Iterator<Item = &'iter mut R> {
        self.contiguous_mut().iter_mut().skip(1)
    }
}

#[derive(Debug)]
pub struct IndexArrayColumn<T: Default> {
    /// These indices are guaranteed to be consistent and are never moved
    /// around to maintain cache locality.
    ///
    /// Each index refers to an index into the `contiguous` data vector.
    ///
    /// Often referred to as "indirect indices".
    indices: Vec<u32>,

    /// The "real" collection. This is contiguous, optimised for cache
    /// locality.
    ///
    /// Each element is a [`Entry`] which, other than the value, also contains
    /// the index of the slot that points to the element.
    contiguous: Vec<Entry<T>>,

    /// Keeps track of free slots of the indirect `indices`.
    free: Vec<u32>,
}

impl<T: Default> Default for IndexArrayColumn<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default> IndexArrayColumn<T> {
    /// Create a blank new Column with a size of `1`.
    ///
    /// The only element present is the degenerate element at index `0`.
    pub fn new() -> Self {
        Self {
            indices: vec![0],
            contiguous: vec![Entry::default()],
            free: Vec::new(),
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
            free: Vec::new(),
        }
    }
}

impl<T: Default> SparseSlot for IndexArrayColumn<T> {
    fn slots_map(&self) -> &Vec<u32> {
        &self.indices
    }

    fn slots_map_mut(&mut self) -> &mut Vec<u32> {
        &mut self.indices
    }

    fn free_list(&self) -> &Vec<u32> {
        &self.free
    }

    fn free_list_mut(&mut self) -> &mut Vec<u32> {
        &mut self.free
    }
}

impl<T: Default> Column<T> for IndexArrayColumn<T> {
    fn len(&self) -> usize {
        self.contiguous.len()
    }

    fn size(&self) -> usize {
        self.indices.len()
    }

    fn free(&mut self, slot: u32) {
        if slot == 0 {
            panic!("slot 0 is reserved for degenerate elements and must not be freed");
        }

        let contiguous_slot = self.indices[slot as usize];
        if contiguous_slot == 0 {
            return;
        }
        self.indices[slot as usize] = 0;

        if let Some(owner_last) = self.contiguous.last().map(Entry::owner) {
            self.indices[owner_last as usize] = contiguous_slot;
        }

        self.contiguous.swap_remove(contiguous_slot as usize);
        self.free.push(slot);
    }

    fn put(&mut self, value: T) -> u32 {
        let index = self.next_slot_index();
        let slot = self.contiguous.len();
        self.indices[index as usize] = slot as u32;
        self.contiguous.push(Entry::new(index, value));
        index
    }
}

impl<'iter, T: Default + 'iter> IterColumn<'iter, T, Entry<T>> for IndexArrayColumn<T> {
    fn contiguous(&self) -> &[Entry<T>] {
        &self.contiguous
    }

    fn contiguous_mut(&mut self) -> &mut [Entry<T>] {
        &mut self.contiguous
    }
}

#[derive(Debug)]
pub struct ArrayColumn<T: Default> {
    /// These indices are guaranteed to be consistent and are never moved
    /// around to maintain cache locality.
    ///
    /// Each index refers to an index into the `contiguous` data vector.
    ///
    /// Often referred to as "indirect indices".
    indices: Vec<u32>,

    /// The "real" collection. This is contiguous, optimised for cache
    /// locality.
    ///
    /// Each element stores directly the value of `T` without any metadata.
    contiguous: Vec<T>,

    /// Keeps track of free slots of the indirect `indices`.
    free: Vec<u32>,
}

impl<T: Default> Default for ArrayColumn<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default> ArrayColumn<T> {
    /// Create a blank new Column with a size of `1`.
    ///
    /// The only element present is the degenerate element at index `0`.
    pub fn new() -> Self {
        Self {
            indices: vec![0],
            contiguous: vec![T::default()],
            free: Vec::new(),
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
        contiguous.push(T::default());

        Self {
            indices: stable_indices,
            contiguous,
            free: Vec::new(),
        }
    }
}

impl<T: Default> SparseSlot for ArrayColumn<T> {
    fn slots_map(&self) -> &Vec<u32> {
        &self.indices
    }

    fn slots_map_mut(&mut self) -> &mut Vec<u32> {
        &mut self.indices
    }

    fn free_list(&self) -> &Vec<u32> {
        &self.free
    }

    fn free_list_mut(&mut self) -> &mut Vec<u32> {
        &mut self.free
    }
}

impl<T: Default> Column<T> for ArrayColumn<T> {
    fn len(&self) -> usize {
        self.contiguous.len()
    }

    fn size(&self) -> usize {
        self.indices.len()
    }

    fn free(&mut self, slot: u32) {
        if slot == 0 {
            panic!("slot 0 is reserved for degenerate elements and must not be freed");
        }

        let contiguous_slot = self.indices[slot as usize];
        if contiguous_slot == 0 {
            return;
        }
        self.indices[slot as usize] = 0;

        self.contiguous.swap_remove(contiguous_slot as usize);
        self.free.push(slot);

        todo!("maintain index stability during ArrayColumn::free");
    }

    fn put(&mut self, value: T) -> u32 {
        let index = self.next_slot_index();
        let slot = self.contiguous.len();
        self.indices[index as usize] = slot as u32;
        self.contiguous.push(value);
        index
    }
}

impl<'iter, T: Default + 'iter> IterColumn<'iter, T, T> for ArrayColumn<T> {
    fn contiguous(&self) -> &[T] {
        &self.contiguous
    }

    fn contiguous_mut(&mut self) -> &mut [T] {
        &mut self.contiguous
    }
}

#[derive(Debug)]
pub struct ParallelIndexArrayColumn<T: Default> {
    /// These indices are guaranteed to be consistent and are never moved
    /// around to maintain cache locality.
    ///
    /// Each index refers to an index into the `contiguous` data vector.
    ///
    /// Often referred to as "indirect indices".
    indices: Vec<u32>,

    /// The "real" collection. This is contiguous, optimised for cache
    /// locality.
    ///
    /// Each element stores directly the value of `T` without any metadata.
    contiguous: Vec<T>,

    /// Keeps track of free slots of the indirect `indices`.
    free: Vec<u32>,

    /// The owner indices of each `T` element. This is parallel to the
    /// `contiguous` vec.
    owners: Vec<u32>,
}

impl<T: Default> Default for ParallelIndexArrayColumn<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default> ParallelIndexArrayColumn<T> {
    /// Create a blank new Column with a size of `1`.
    ///
    /// The only element present is the degenerate element at index `0`.
    pub fn new() -> Self {
        Self {
            indices: vec![0],
            contiguous: vec![T::default()],
            owners: vec![0],
            free: Vec::new(),
        }
    }

    /// Creata a blank new column with the given `capacity`.
    ///
    /// All elements are initialised with their [`Default`] implementation.
    /// This includes the degenerate element at index `0`.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut stable_indices = Vec::with_capacity(capacity);
        let mut contiguous = Vec::with_capacity(capacity);
        let mut owners = Vec::with_capacity(capacity);

        stable_indices.push(0);
        contiguous.push(T::default());
        owners.push(0);

        Self {
            indices: stable_indices,
            contiguous,
            owners,
            free: Vec::new(),
        }
    }

    pub fn handles(&self) -> &[u32] {
        &self.owners
    }
}

impl<T: Default> SparseSlot for ParallelIndexArrayColumn<T> {
    fn slots_map(&self) -> &Vec<u32> {
        &self.indices
    }

    fn slots_map_mut(&mut self) -> &mut Vec<u32> {
        &mut self.indices
    }

    fn free_list(&self) -> &Vec<u32> {
        &self.free
    }

    fn free_list_mut(&mut self) -> &mut Vec<u32> {
        &mut self.free
    }
}

impl<T: Default> Column<T> for ParallelIndexArrayColumn<T> {
    fn len(&self) -> usize {
        self.contiguous.len()
    }

    fn size(&self) -> usize {
        self.indices.len()
    }

    fn free(&mut self, slot: u32) {
        if slot == 0 {
            panic!("slot 0 is reserved for degenerate elements and must not be freed");
        }

        let contiguous_slot = self.indices[slot as usize];
        if contiguous_slot == 0 {
            return;
        }

        self.indices[slot as usize] = 0;
        let last_owner = *self
            .owners
            .last()
            .expect("contiguous vectors are never empty");
        self.indices[last_owner as usize] = contiguous_slot;

        self.owners.swap_remove(contiguous_slot as usize);
        self.contiguous.swap_remove(contiguous_slot as usize);
        self.free.push(slot);
    }

    fn put(&mut self, value: T) -> u32 {
        let index = self.next_slot_index();
        let slot = self.contiguous.len();
        self.indices[index as usize] = slot as u32;
        self.contiguous.push(value);
        self.owners.push(index);
        index
    }
}

impl<'iter, T: Default + 'iter> IterColumn<'iter, T, T> for ParallelIndexArrayColumn<T> {
    fn contiguous(&self) -> &[T] {
        &self.contiguous
    }

    fn contiguous_mut(&mut self) -> &mut [T] {
        &mut self.contiguous
    }
}

impl<T: Default> IntoIterator for IndexArrayColumn<T> {
    type Item = Entry<T>;

    type IntoIter = std::vec::IntoIter<Entry<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.contiguous.into_iter()
    }
}

impl<T: Default> IntoIterator for ArrayColumn<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.contiguous.into_iter()
    }
}

impl<T: Default> IntoIterator for ParallelIndexArrayColumn<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.contiguous.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_last_after_random_free() {
        let mut column = ParallelIndexArrayColumn::<u32>::new();

        for i in 0..50 {
            column.put(i as u32);
        }
        let last = column.put(100);

        // free random
        {
            column.free(37);
            column.free(14);
            column.free(32);
            column.free(45);
            column.free(24);
            column.free(3);
            column.free(7);
            column.free(35);
        }

        // free last
        column.free(last);
    }
}

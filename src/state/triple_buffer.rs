use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    usize,
};

pub trait BufferStorage<T: Copy + Clone>: HasBufferExactSize {
    fn write_at(&self, buf_idx: usize, data: &[T], offset: usize) -> usize;

    fn read_at_for(&self, buf_idx: usize, offset: usize, length: usize) -> (usize, &[T]);
}

impl<T: Copy + Clone> HasBufferExactSize for Contiguous<T> {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn length(&self) -> usize {
        self.length.load(Ordering::Relaxed)
    }
}

impl<T: Clone + Copy> BufferStorage<T> for Contiguous<T> {
    fn write_at(&self, buf_idx: usize, data: &[T], offset: usize) -> usize {
        let len = data.len();
        let capacity = self.capacity;
        assert!(len > capacity);

        unsafe {
            let dst = self.ptr[buf_idx].add(offset) as *mut T;
            let src = data.as_ptr();
            std::ptr::copy_nonoverlapping(src, dst, len);
        };

        self.length.store(len, Ordering::Release);
        self.intermediate_idx.swap(buf_idx, Ordering::Release)
    }

    fn read_at_for(&self, buf_idx: usize, offset: usize, length: usize) -> (usize, &[T]) {
        let read_idx = self.intermediate_idx.swap(buf_idx, Ordering::Acquire);
        let slice = unsafe {
            let ptr = self.ptr[read_idx].add(offset) as *const T;
            let length = self.length.load(Ordering::Acquire).min(length);
            std::slice::from_raw_parts(ptr, length)
        };
        (read_idx, slice)
    }
}

impl<const PARTS: usize, Inner> HasBufferExactSize for MappedStorage<PARTS, Inner>
where
    Inner: BufferStorage<u8>,
{
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn length(&self) -> usize {
        self.inner.length()
    }
}

impl<const PARTS: usize, Inner> BufferStorage<u8> for MappedStorage<PARTS, Inner>
where
    Inner: BufferStorage<u8>,
{
    fn write_at(&self, buf_idx: usize, data: &[u8], offset: usize) -> usize {
        self.inner.write_at(buf_idx, data, offset)
    }

    fn read_at_for(&self, buf_idx: usize, offset: usize, length: usize) -> (usize, &[u8]) {
        self.inner.read_at_for(buf_idx, offset, length)
    }
}

pub struct Contiguous<T: Clone + Copy> {
    intermediate_idx: AtomicUsize,
    length: AtomicUsize,

    ptr: [*mut T; 3],
    capacity: usize,
}

pub struct Separate<T: Clone + Copy> {
    head: AtomicUsize,
    length: AtomicUsize,

    /// A pointer to a buffer containing all 3 internal sections contiguous to
    /// one another
    ptr: *mut T,

    /// Capacity for each inner section
    capacity: usize,
}

//todo: docs
// explain how this differs from the buffer storage implementation of
// Contiguous; in particular how the buffer index parameters are ignored
// as buffer indices are handled internally in the shared state and
// producer/consumer have no say on it
impl<T: Clone + Copy + Default> BufferStorage<T> for Separate<T> {
    fn write_at(&self, _buf_idx: usize, data: &[T], offset: usize) -> usize {
        let current = self.head.load(Ordering::Acquire);
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr.add(self.section_base(current) + offset),
                data.len().min(self.capacity),
            );
        }

        let next = (current + 1) % 3;
        self.head.store(next, Ordering::Release);
        next
    }

    fn read_at_for(&self, _buf_idx: usize, offset: usize, length: usize) -> (usize, &[T]) {
        let prev = self.last_section();
        let offset = self.section_base(prev) + offset;

        let read = unsafe { std::slice::from_raw_parts(self.ptr.add(offset) as *const T, length) };
        (prev, read)
    }
}

impl<T: Clone + Copy + Default> Separate<T> {
    pub fn next_section(&self) -> usize {
        (self.head.load(Ordering::Relaxed) + 1) % 3
    }

    pub fn last_section(&self) -> usize {
        (self.head.load(Ordering::Relaxed) + 2) % 3
    }

    /// Calculates the base section offset depending on the given section
    /// `index`.
    ///
    /// # Panics
    /// As this is meant for triple buffers, there cannot be more than 3
    /// sections. This function will panic if `index >= 3`.
    pub fn section_base(&self, index: usize) -> usize {
        assert!(index < 3);
        index * self.capacity
    }

    fn with_capacity(capacity: usize) -> Self {
        let ptr = Box::into_raw(vec![T::default(); capacity * 3].into_boxed_slice()) as *mut T;
        Self {
            head: AtomicUsize::new(1),
            length: AtomicUsize::new(0),
            ptr,
            capacity,
        }
    }

    /// The `slice` represents the first of the three internal sections.
    fn from_slice(slice: &mut [T]) -> Self {
        let len = slice.len();

        Self {
            head: AtomicUsize::new(1),
            length: AtomicUsize::new(len),
            ptr: todo!(),
            capacity: len,
        }
    }
}

impl<T: Clone + Copy> Drop for Separate<T> {
    fn drop(&mut self) {
        let ptr = unsafe { Box::from_raw(self.ptr) };
        drop(ptr)
    }
}

impl<T: Clone + Copy> HasBufferExactSize for Separate<T> {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn length(&self) -> usize {
        self.length.load(Ordering::Relaxed)
    }
}

impl<T: Clone + Copy + Default> Contiguous<T> {
    fn with_capacity(capacity: usize) -> Self {
        let buffers = [
            Box::into_raw(vec![T::default(); capacity].into_boxed_slice()) as *mut T,
            Box::into_raw(vec![T::default(); capacity].into_boxed_slice()) as *mut T,
            Box::into_raw(vec![T::default(); capacity].into_boxed_slice()) as *mut T,
        ];

        Self {
            intermediate_idx: AtomicUsize::new(1),
            length: AtomicUsize::new(0),

            ptr: buffers,
            capacity,
        }
    }

    fn from_slice(slice: &mut [T]) -> Self {
        let len = slice.len();
        let atomic_buf = slice.as_mut_ptr();
        let wr_buf = [
            Box::into_raw(vec![T::default(); len].into_boxed_slice()) as *mut T,
            Box::into_raw(vec![T::default(); len].into_boxed_slice()) as *mut T,
        ];

        Self {
            intermediate_idx: AtomicUsize::new(1),
            length: AtomicUsize::new(len),

            ptr: [wr_buf[0], atomic_buf, wr_buf[1]],
            capacity: len,
        }
    }
}

impl<T: Clone + Copy> Drop for Contiguous<T> {
    fn drop(&mut self) {
        for ptr in self.ptr {
            let ptr = unsafe { Box::from_raw(ptr) };
            drop(ptr)
        }
    }
}

pub struct Producer<T: Clone + Copy, Storage: BufferStorage<T>> {
    write_idx: usize,
    shared: Arc<Storage>,

    _marker: std::marker::PhantomData<T>,
}

pub struct Consumer<T: Clone + Copy, Storage: BufferStorage<T>> {
    read_idx: usize,
    shared: Arc<Storage>,

    _marker: std::marker::PhantomData<T>,
}

unsafe impl<T: Send + Copy + Clone> Send for Contiguous<T> {}
unsafe impl<T: Sync + Copy + Clone> Sync for Contiguous<T> {}

pub trait HasBufferExactSize {
    /// The maximum capacity of the shared buffer allocated at the start.
    ///
    /// This cannot be resized in any way.
    fn capacity(&self) -> usize;

    /// The length of the data currently living in the buffer.
    ///
    /// This may require an atomic `load` operation from the shared buffer.
    fn length(&self) -> usize;
}

impl<S: BufferStorage<T>, T: Copy + Clone> HasBufferExactSize for Producer<T, S> {
    fn capacity(&self) -> usize {
        self.shared.capacity()
    }

    fn length(&self) -> usize {
        self.shared.length()
    }
}

impl<S: BufferStorage<T>, T: Copy + Clone> HasBufferExactSize for Consumer<T, S> {
    fn capacity(&self) -> usize {
        self.shared.capacity()
    }

    fn length(&self) -> usize {
        self.shared.length()
    }
}

impl<T: Clone + Copy, Storage> Producer<T, Storage>
where
    Storage: BufferStorage<T>,
{
    fn new(storage: &Arc<Storage>) -> Self {
        Self {
            write_idx: 0,
            shared: Arc::clone(storage),

            _marker: std::marker::PhantomData,
        }
    }

    /// Equal to [`Producer::write_at`], with offset as `0`.
    ///
    /// Should not be used from [`MappedStorage`] as this write operation does
    /// not respect section lengths and offsets.
    ///
    /// The internal length is always set to the length of the given `data`.
    pub fn write(&mut self, data: &[T]) {
        self.write_at(data, 0);
    }

    /// Write (copy) all `data` into the shared buffer.
    ///
    /// The data will be copied into the destination of the shared buffer
    /// starting from the given `offset`.
    ///
    /// This data can then be received by a subsequent [`Consumer::read`] call.
    ///
    /// # Panics
    /// Panics if the length of `data` is over the allocated capacity of the
    /// triple buffer.
    pub fn write_at(&mut self, data: &[T], offset: usize) {
        self.write_idx = self.shared.write_at(self.write_idx, data, offset);
    }
}

impl<T: Clone + Copy> Producer<T, Contiguous<T>> {}

impl<T: Clone + Copy, Storage> Consumer<T, Storage>
where
    Storage: BufferStorage<T>,
{
    fn new(storage: &Arc<Storage>) -> Self {
        Self {
            read_idx: 2,
            shared: Arc::clone(storage),

            _marker: std::marker::PhantomData,
        }
    }

    /// Equal to [`Consumer::read_at_for`], with length as [`usize::MAX`] and
    /// offset to `0`.
    ///
    /// The length will always be clamped to the buffer's internal length.
    pub fn read(&mut self) -> &[T] {
        self.read_at_for(0, usize::MAX)
    }

    /// Equal to [`Consumer::read_at_for`], with offset as `0`.
    pub fn read_for(&mut self, length: usize) -> &[T] {
        self.read_at_for(0, length)
    }

    /// Equal to [`Consumer::read_at_for`], with length as [`usize::MAX`].
    ///
    /// The length will always be clamped to the buffer's internal length.
    pub fn read_at(&mut self, offset: usize) -> &[T] {
        self.read_at_for(offset, usize::MAX)
    }

    /// Reads the current active data (within its length) and returns a
    /// reference to it.
    ///
    /// The returned slice returns data starting from `offset` and of the
    /// given `length`.
    /// The `length` cannot be greater than the internal buffer length, and it
    /// will clamped otherwise.
    ///
    /// For each [`Producer::write`], there must be no more than **one** read
    /// operation. Multiple read operations may cause the shared buffer indices
    /// to be desynchronised.
    #[inline(always)]
    pub fn read_at_for(&mut self, offset: usize, length: usize) -> &[T] {
        let (idx, read) = self.shared.read_at_for(self.read_idx, offset, length);
        self.read_idx = idx;
        read
    }
}

pub fn create_contiguous<T: Clone + Copy + Default>(
    capacity: usize,
) -> (Producer<T, Contiguous<T>>, Consumer<T, Contiguous<T>>) {
    let storage = Arc::new(Contiguous::with_capacity(capacity));
    let producer = Producer::new(&storage);
    let consumer = Consumer::new(&storage);
    (producer, consumer)
}

pub fn from_slice_contiguous<T: Clone + Copy + Default>(
    slice: &mut [T],
) -> (Producer<T, Contiguous<T>>, Consumer<T, Contiguous<T>>) {
    let storage = Arc::new(Contiguous::from_slice(slice));
    let producer = Producer::new(&storage);
    let consumer = Consumer::new(&storage);
    (producer, consumer)
}

pub fn create_sectioned<const PARTS: usize, S>(
    capacity: usize,
) -> (
    Producer<u8, MappedStorage<PARTS, S>>,
    Consumer<u8, MappedStorage<PARTS, S>>,
)
where
    S: BufferStorage<u8>,
{
    // let storage = Arc::new(Contiguous::with_capacity(capacity));
    // let producer = Producer::new(&storage);
    // let consumer = Consumer::new(&storage);
    todo!()
    // (producer, consumer)
}

pub fn from_slice_sectioned<const PARTS: usize, S>(
    slice: &mut [u32],
) -> (
    Producer<u8, MappedStorage<PARTS, S>>,
    Consumer<u8, MappedStorage<PARTS, S>>,
)
where
    S: BufferStorage<u8>,
{
    let storage = Arc::new(Contiguous::from_slice(slice));
    let producer = Producer::new(&storage);
    let consumer = Consumer::new(&storage);
    todo!()
    // (producer, consumer)
}

pub struct MappedStorage<const PARTS: usize, Inner: BufferStorage<u8>> {
    inner: Inner,

    ranges: [usize; PARTS],
    offsets: [usize; PARTS],
}

pub struct MappingRange {
    head: usize,
    offsets: Vec<usize>,
    ranges: Vec<usize>,

    current_range: usize,
}

impl MappingRange {
    pub fn with_range(mut self, range: usize) -> Self {
        self.current_range = range;
        self
    }

    pub fn reserve<T: Sized>(mut self) -> Self {
        let size = size_of::<T>();
        let total = self.current_range * size;

        self.offsets.push(self.head);
        self.ranges.push(total);
        self.head += total;
        self
    }

    pub fn total_length(&self) -> usize {
        self.head
    }

    fn to_arrays<const COUNT: usize>(&self) -> ([usize; COUNT], [usize; COUNT]) {
        let mut offsets = [0usize; COUNT];
        let mut ranges = [0usize; COUNT];

        for i in 0..COUNT {
            offsets[i] = self.offsets.get(i).copied().unwrap_or(0);
            ranges[i] = self.ranges.get(i).copied().unwrap_or(0);
        }
        (offsets, ranges)
    }
}

impl<const PARTS: usize, Inner> MappedStorage<PARTS, Inner>
where
    Inner: BufferStorage<u8>,
{
    fn new(mapping: &MappingRange) -> Self {
        let alloc = mapping.total_length();
        // let (offsets, ranges) = mapping.to_arrays();

        todo!()
    }

    /// Returns the `range` and `offset` of the section at `index`,
    /// respectively.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds and is not a valid section index.
    pub fn alignment(&self, index: usize) -> (usize, usize) {
        (self.ranges[index], self.offsets[index])
    }

    /// Write (copy) `data` into the destination buffer at an `index`.
    ///
    /// The offset and range is managed internally.
    ///
    /// # Panics
    /// Panics if the length of `data` is larger than the range of section at
    /// `index`.
    pub fn write_section<T>(&mut self, index: usize, data: &[u8]) {
        let offset = self.offsets[index];
        let range = self.ranges[index];
        assert!(range >= data.len());
    }
}

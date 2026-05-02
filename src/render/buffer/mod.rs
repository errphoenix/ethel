pub mod immutable;
pub mod layout;
pub mod partitioned;

use std::sync::atomic::{AtomicU32, Ordering};

pub use immutable::{ImmutableBuffer, UninitImmutableBuffer};
pub use layout::Layout;
pub use partitioned::PartitionedTriBuffer;

#[derive(Clone, Copy, Debug)]
pub enum InitStrategy<T: Sized + Clone, F: Fn() -> T> {
    Zero,
    FillWith(F),
}

macro_rules! assert_tb_section {
    ($s:expr) => {
        let s = $s;
        assert!(
            s < 3,
            "attempted to access section {s} in a triple buffer (=3 sections)"
        );
    };
}

pub(crate) use assert_tb_section;

/// A triple buffered OpenGL buffer over multiple memory blocks.
///
/// Unlike [`PartitionedTriBuffer`], this buffer is made for only one type, and
/// each triple buffer section is a dinstict OpenGL buffer.
///
/// This is useful for OpenGL indexed buffers, such as indirect command
/// buffers and array buffers, that do not support `glBindBufferRange` (which
/// [`PartitionedTriBuffer`] depends on).
///
/// This is also the reason as to why multiple types (parts) are not supported
/// in [`TriBuffer`].
///
/// <div class="warning">
///
/// ### Note
///
/// Reading from the GPU buffers is slower than reading from system memory,
/// thus it is not recommended to mutate data through the `view_section_mut`
/// functions in performance-critical scenarios.
///
/// Prefer the usage of `blit_section` to mutate data as these correspond to a
/// single `memcpy` operation directly to the underlying memory, which is
/// significantly faster because the required modification is reduced to a
/// single operation. They're also not unsafe, unlike `view_section_mut`.
///
/// This is also valid for [`PartitionedTriBuffer`].
///
/// </div>
///
/// [`PartitionedTriBuffer`]: partitioned::PartitionedTriBuffer
#[derive(Default, Debug)]
pub struct TriBuffer<T: Sized + Clone + Copy> {
    lengths: [AtomicU32; 3],
    gl_obj: [u32; 3],
    ptr: [*mut T; 3],

    /// Capacity per each section. This is number of elements.
    capacity: usize,

    _marker: std::marker::PhantomData<T>,
}

unsafe impl<T> Sync for TriBuffer<T> where T: Sized + Clone + Copy {}
unsafe impl<T> Send for TriBuffer<T> where T: Sized + Clone + Copy {}

impl<T> TriBuffer<T>
where
    T: Sized + Clone + Copy,
{
    pub fn zeroed(capacity: usize) -> Self {
        Self::new(capacity, InitStrategy::<T, fn() -> T>::Zero)
    }

    pub fn new<F: Fn() -> T>(capacity: usize, init: InitStrategy<T, F>) -> Self {
        let mut gl_obj = [0; 3];
        let mut ptr = [std::ptr::null_mut(); 3];
        let total_size = (capacity * size_of::<T>()) as isize;

        unsafe {
            janus::gl::CreateBuffers(1, &mut gl_obj[0]);
            janus::gl::CreateBuffers(1, &mut gl_obj[1]);
            janus::gl::CreateBuffers(1, &mut gl_obj[2]);

            let flags = janus::gl::MAP_WRITE_BIT
                | janus::gl::MAP_READ_BIT
                | janus::gl::MAP_COHERENT_BIT
                | janus::gl::MAP_PERSISTENT_BIT;

            for i in 0..3 {
                janus::gl::NamedBufferStorage(gl_obj[i], total_size, std::ptr::null(), flags);
                ptr[i] = janus::gl::MapNamedBufferRange(gl_obj[i], 0, total_size, flags) as *mut T;
            }
        }

        match init {
            InitStrategy::Zero => {
                for i in 0..3 {
                    unsafe {
                        janus::gl::ClearNamedBufferData(
                            gl_obj[i],
                            janus::gl::R32UI,
                            janus::gl::RED_INTEGER,
                            janus::gl::UNSIGNED_INT,
                            std::ptr::null(),
                        );
                    }
                }
            }
            InitStrategy::FillWith(func) => {
                for i in 0..3 {
                    let ptr = ptr[i];
                    for j in 0..capacity {
                        unsafe {
                            std::ptr::write(ptr.add(j), func());
                        }
                    }
                }
            }
        }

        Self {
            lengths: [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)],
            gl_obj,
            ptr,
            capacity,
            _marker: std::marker::PhantomData,
        }
    }

    /// Binds the specified `section` of the tri-buffer to the given
    /// `ssbo_index`, with a custom `offset`.
    ///
    /// # Panic
    /// If `section` is not a value within the range (0, 2).
    /// Or if `offset` is greater or equal to the buffer's internal length.
    pub fn bind_shader_storage(&self, section: usize, ssbo_index: usize, offset: u32) {
        assert_tb_section!(section);

        #[cfg(debug_assertions)]
        {
            let ssbo_align =
                unsafe { janus::gl::GL_SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT } as usize;
            assert_eq!(self.capacity % ssbo_align, 0)
        }

        let base_length = self.lengths[section].load(Ordering::Relaxed);

        assert!(
            base_length >= offset,
            "offset cannot be greater or equal to buffer length {base_length}"
        );

        let offset_bytes = offset as usize * size_of::<T>();
        let length_bytes = (base_length - offset) as usize * size_of::<T>();

        unsafe {
            janus::gl::BindBufferRange(
                janus::gl::SHADER_STORAGE_BUFFER,
                ssbo_index as u32,
                self.gl_obj[section],
                offset_bytes as isize,
                length_bytes as isize,
            );
        }
    }

    pub fn view_section(&self, section: usize) -> View<'_, T> {
        assert_tb_section!(section);

        let ptr = self.ptr[section];
        let len = self.lengths[section].load(Ordering::Relaxed);
        let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };

        View {
            slice,
            offset: 0,
            length: len,
            source: self.gl_obj[section],
        }
    }

    pub fn view_section_mut(&self, section: usize) -> ViewMut<'_, T> {
        assert_tb_section!(section);

        let ptr = self.ptr[section];
        let len = self.lengths[section].load(Ordering::Relaxed);
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len as usize) };

        ViewMut {
            slice,
            offset: 0,
            length: len,
            source: self.gl_obj[section],
        }
    }

    pub fn set_section_length(&self, section: usize, length: u32) {
        assert_tb_section!(section);
        assert!(
            self.capacity as u32 > length,
            "attempted to set length of section {section} to {length} but with capacity {}",
            self.capacity
        );

        self.lengths[section].store(length, Ordering::Release);
    }

    pub fn section_length(&self, section: usize) -> u32 {
        assert_tb_section!(section);
        self.lengths[section].load(Ordering::Relaxed)
    }

    /// Copy the given `data` into a `section` of the triple buffer at a given
    /// `offset`.
    ///
    /// This is the equivalent of a `memcpy` operation.
    ///
    /// The given `offset` must be the amount of elements `T` to skip inside
    /// of the buffer, not bytes.
    ///
    /// If the length of `data` exceeds the capacity of the buffer, it will be
    /// automatically clamped and any exceeding elements will be ignored.
    ///
    /// # Panics
    /// * If `section` is not a value within the range (0, 2).
    /// * If `offset` is greater than the length of the section.
    pub fn blit_section(&mut self, section: usize, data: &[T], offset: usize) {
        assert_tb_section!(section);
        assert!(
            self.capacity > offset,
            "attempted to blit at offset {offset} with section capacity {}",
            self.capacity
        );

        let src = data.as_ptr();
        let avail = self.capacity - offset;
        let len = avail.min(data.len());
        self.lengths[section].store(len as u32, Ordering::Release);

        unsafe {
            std::ptr::copy_nonoverlapping(src, self.ptr[section].add(offset), len);
        }
    }

    /// Copy the given `data` into a `section` of the triple buffer at a given
    /// `offset` with a padding of `pad_lan` at the end of each
    /// element.
    ///
    /// If the length of `data` exceeds the capacity of the buffer, it will be
    /// automatically clamped and any exceeding elements will be ignored.
    ///
    /// This function is intended for operations where the CPU and GPU data
    /// representations differ due to memory alignment requirements.
    ///
    /// Note that this operation is likely slower than the standard
    /// [`blit_section`].
    ///
    /// It is, in most cases, not recommended and [`blit_section`] should be
    /// preferred if possible.
    ///
    /// # Motivation
    /// Imagine you want to pass a position vector to the GPU: this is a
    /// 3-dimensional vector on the CPU, but it must be a vec4 on the GPU due
    /// to OpenGL's SSBO alignment requirements.
    ///
    /// In most cases, to avoid this issue, you would likely settle for an
    /// intermediary buffer on the CPU where this conversion happens or you
    /// could simply just store all positions as a 4-dimensional vector on the
    /// CPU (maybe intelligently packing relevant data on the W component) if
    /// it is not performance critical.
    ///
    /// In some cases, though, like visualising physics data, using a
    /// 4-dimensional is not an option as it would pollute the CPU cache with
    /// an unused float in a very performance critical scenario.
    ///
    /// This is the reason this function exists: it will pad out each element
    /// of `data` with the given `pad_len` in bytes to satisfy SSBO alignment
    /// requirements, without the need of intermediary buffers on the CPU.
    ///
    /// # Panics
    /// * If `section` is not a value within the range (0, 2).
    /// * If `offset` is greater than the length of the section.
    /// * If the size of the given type `S` + `pad_len` does not match the size
    ///   of the buffer type `T`.
    /// * If `pad_len` is 0.
    ///
    /// [`blit_section`]: TriBuffer::blit_section
    pub fn blit_section_padded<S: Clone + Copy + Default>(
        &mut self,
        section: usize,
        data: &[S],
        offset: usize,
        pad_len: usize,
    ) {
        assert_ne!(
            pad_len, 0,
            "cannot blit with padding: invalid padding value of 0"
        );

        assert_tb_section!(section);
        assert!(
            self.capacity > offset,
            "attempted to blit at offset {offset} with section length {}",
            self.capacity
        );

        let avail = self.capacity * size_of::<T>() - offset;
        let data_bytes_padded = size_of::<S>() + pad_len;
        assert_eq!(
            data_bytes_padded,
            size_of::<T>(),
            "cannot blit with padding: expected type size of {} bytes (T), but got: {} bytes (S) + {pad_len} bytes (padding) = {data_bytes_padded} bytes",
            size_of::<T>(),
            size_of::<S>(),
        );

        let avail_count = avail / data_bytes_padded;
        let data_count = data.len();

        // safe total length of data, element count
        let data_len = avail_count.min(data_count);
        self.lengths[section].store(data_len as u32, Ordering::Release);

        // SAFETY: we assert the section and partition are valid within this
        // buffer's layout. The buffer's layout, in turn, guarantees valid
        // base offsets and base lengths.
        // The caller guarantees the pointer to `data` must always be valid.
        // Additionally, the caller must also ensure that that the length of
        // T + `pad_len` correspond to the size of the type on the GPU.
        unsafe {
            let mut dst = (self.ptr[section] as *mut u8).add(offset);
            for i in 0..data_len {
                std::ptr::write_unaligned(dst as *mut S, data[i]);
                dst = dst.add(size_of::<S>());
                dst.write_bytes(0, pad_len);
                dst = dst.add(pad_len);
            }
        }
    }
}

impl<T> Drop for TriBuffer<T>
where
    T: Sized + Clone + Copy,
{
    fn drop(&mut self) {
        unsafe {
            for i in 0..3 {
                janus::gl::UnmapNamedBuffer(self.gl_obj[i]);
            }
            janus::gl::DeleteBuffers(3, self.gl_obj.as_ptr());
        }
        self.ptr = [std::ptr::null_mut(); 3];
    }
}

#[derive(Debug)]
pub struct View<'buf, T: Sized> {
    slice: &'buf [T],
    offset: u32,
    length: u32,
    source: u32,
}

impl<'buf, T: Sized> View<'buf, T> {
    pub const fn as_ptr(&self) -> *const T {
        self.slice.as_ptr()
    }

    pub const fn as_slice(&self) -> &'buf [T] {
        self.slice
    }

    /// The original offset of the data in the buffer it belongs to.
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// The length in bytes.
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// The original OpenGL buffer object this view belongs to.
    pub const fn source(&self) -> u32 {
        self.source
    }
}

impl<T> View<'_, T>
where
    T: Sized + Clone,
{
    pub fn to_vec(&self) -> Vec<T> {
        self.slice.to_vec()
    }
}

impl<T> ViewMut<'_, T>
where
    T: Sized + Clone,
{
    pub fn to_vec(&self) -> Vec<T> {
        self.slice.to_vec()
    }
}

impl<T: Sized> std::ops::Deref for View<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<T: Sized> std::ops::Deref for ViewMut<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<T: Sized> std::ops::DerefMut for ViewMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.slice
    }
}

#[derive(Debug)]
pub struct ViewMut<'buf, T: Sized> {
    slice: &'buf mut [T],
    offset: u32,
    length: u32,
    source: u32,
}

impl<'buf, T: Sized> ViewMut<'buf, T> {
    pub const fn as_ptr(&self) -> *const T {
        self.slice.as_ptr()
    }

    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self.slice.as_mut_ptr()
    }

    pub const fn as_mut_slice(&'buf mut self) -> &'buf mut [T] {
        self.slice
    }

    pub fn as_slice(&'buf self) -> &'buf [T] {
        self.slice.as_ref()
    }

    /// The original offset of the data in the buffer it belongs to.
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// The length in bytes.
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// The original OpenGL buffer object. this view belongs to.
    pub const fn source(&self) -> u32 {
        self.source
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageSection {
    Front = StorageSection::FRONT_BYTE,
    Back = StorageSection::BACK_BYTE,
    Spare = StorageSection::SPARE_BYTE,
}

impl StorageSection {
    const FRONT_BYTE: u8 = 0b00000001;
    const BACK_BYTE: u8 = 0b00001000;
    const SPARE_BYTE: u8 = 0b01000000;

    pub fn from_byte(byte: u8) -> Self {
        match byte {
            Self::FRONT_BYTE => Self::Front,
            Self::BACK_BYTE => Self::Back,
            Self::SPARE_BYTE => Self::Spare,
            _ => panic!(
                r#"{byte} is not a valid storage section byte, valid options are: {} (front), {} (back), {} (spare)"#,
                Self::FRONT_BYTE,
                Self::BACK_BYTE,
                Self::SPARE_BYTE
            ),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Front => Self::Back,
            Self::Back => Self::Spare,
            Self::Spare => Self::Front,
        }
    }

    pub fn advance(&mut self) {
        *self = self.next();
    }

    pub fn as_index(&self) -> usize {
        match self {
            Self::Front => 0,
            Self::Back => 1,
            Self::Spare => 2,
        }
    }
}

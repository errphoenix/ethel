pub mod layout;
pub mod partitioned;

pub use layout::Layout;
pub use partitioned::PartitionedTriBuffer;

#[derive(Clone, Copy, Debug)]
pub enum InitStrategy<T: Sized + Clone, F: Fn() -> T> {
    Zero,
    FillWith(F),
}

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
/// [`PartitionedTriBuffer`]: partitioned::PartitionedTriBuffer
#[derive(Clone, Default, Debug)]
pub struct TriBuffer<T: Sized + Clone> {
    gl_obj: [u32; 3],
    ptr: [*mut T; 3],
    capacity: usize,

    _marker: std::marker::PhantomData<T>,
}

unsafe impl<T> Sync for TriBuffer<T> where T: Sized + Clone {}
unsafe impl<T> Send for TriBuffer<T> where T: Sized + Clone {}

impl<T> TriBuffer<T>
where
    T: Sized + Clone,
{
    pub fn new<F: Fn() -> T>(capacity: usize, init: InitStrategy<T, F>) -> Self {
        let mut gl_obj = [0; 3];
        let mut ptr = [std::ptr::null_mut(); 3];
        let total_size = (capacity * size_of::<T>()) as isize;

        unsafe {
            janus::gl::CreateBuffers(3, gl_obj.as_mut_ptr());

            let flags = janus::gl::MAP_WRITE_BIT
                | janus::gl::MAP_COHERENT_BIT
                | janus::gl::MAP_PERSISTENT_BIT;
            for i in 0..3 {
                janus::gl::NamedBufferStorage(
                    gl_obj[i],
                    total_size,
                    std::ptr::null(),
                    flags | janus::gl::DYNAMIC_STORAGE_BIT,
                );
                ptr[i] = janus::gl::MapNamedBuffer(gl_obj[i], flags) as *mut T;
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
            gl_obj,
            ptr,
            capacity,
            _marker: std::marker::PhantomData,
        }
    }

    /// Binds the specified `section` of the tri-buffer to the given
    /// `ssbo_index`.
    ///
    /// # Panic
    /// If `section` is not a value within the range (0, 2).
    pub fn bind_shader_storage(&self, section: usize, ssbo_index: usize) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        unsafe {
            janus::gl::BindBufferBase(
                janus::gl::SHADER_STORAGE_BUFFER,
                ssbo_index as u32,
                self.gl_obj[section],
            );
        }
    }

    pub fn view_section(&self, section: usize) -> View<'_, T> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let ptr = self.ptr[section];
        let slice = unsafe { std::slice::from_raw_parts(ptr, self.capacity) };
        View {
            slice,
            offset: 0,
            length: self.capacity as u32,
            source: self.gl_obj[section],
        }
    }

    pub fn view_section_mut(&self, section: usize) -> ViewMut<'_, T> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let ptr = self.ptr[section];
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, self.capacity) };
        ViewMut {
            slice,
            offset: 0,
            length: self.capacity as u32,
            source: self.gl_obj[section],
        }
    }

    pub fn blit_section(&self, section: usize, data: &[T]) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let src = data.as_ptr();
        let len = self.capacity;

        unsafe {
            std::ptr::copy_nonoverlapping(src, self.ptr[section], len);
        }
    }
}

impl<T> Drop for TriBuffer<T>
where
    T: Sized + Clone,
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

    /// The original OpenGL buffer object. this view belongs to.
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

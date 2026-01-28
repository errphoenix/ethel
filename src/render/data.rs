use std::{
    ptr,
    sync::atomic::{AtomicU8, Ordering},
};

use glam::usize;
use janus::gl::{self, types::__GLsync};

pub trait GlPropertyEnum {
    fn as_gl_enum(&self) -> u32;
}

#[derive(Clone, Debug)]
pub struct Layout<const PARTS: usize> {
    head: usize,
    last: usize,
    offsets: [usize; PARTS],
    lengths: [usize; PARTS],
    shader: [u32; PARTS],
}

impl<const PARTS: usize> Default for Layout<PARTS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const PARTS: usize> Layout<PARTS> {
    pub fn new() -> Self {
        assert!(PARTS != 0);
        Self {
            head: 0,
            last: 0,
            offsets: [0; PARTS],
            lengths: [0; PARTS],
            shader: [u32::MAX; PARTS],
        }
    }

    pub fn section<T: Sized>(mut self, count: usize) -> Self {
        let head = self.head;
        assert!(head < PARTS, "layout only allows {PARTS} sections");
        let length = size_of::<T>() * count;

        let alignment = unsafe { janus::gl::GL_SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT } as usize;
        let offset = (self.last + alignment - 1) & !(alignment - 1);
        self.offsets[head] = offset;
        self.lengths[head] = length;

        self.last = length + offset;
        self.head += 1;

        self
    }

    pub fn with_shader_storage(mut self, binding: u32) -> Self {
        self.shader[self.head - 1] = binding;
        self
    }

    pub fn offset_at(&self, index: usize) -> usize {
        self.offsets[index]
    }

    pub fn length_at(&self, index: usize) -> usize {
        self.lengths[index]
    }

    pub fn ssbo_of(&self, index: usize) -> Option<u32> {
        let binding = self.shader[index];
        if binding != u32::MAX {
            Some(binding)
        } else {
            None
        }
    }

    /// Returns the aligned total length of all parts and their lengths.
    ///
    /// This is aligned to OpenGL's SSBO [`alignment offset requirement`],
    /// through [`janus::gl::align_to_gl_ssbo`].
    ///
    /// This is **REQUIRED** for GL operations such as `glBindBufferRange`.
    /// Using a non-aligned offset (directly accessing `last` from [`Layout`])
    /// will lead to undefined behaviour in GL operations.
    ///
    /// [`alignment offset requirement`]: janus::gl::GL_SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT
    pub fn len(&self) -> usize {
        janus::align_to_gl_ssbo(self.last as i32) as usize
    }
}

/// A triple buffer OpenGL buffer over a single memory block.
///
/// This handles alignments and offsets of each memory section and part (a
/// contiguous memory block of data of the same type).
///
/// # OpenGL Representation
/// The GPU buffers are coherent persistent copy-write buffers. It includes
/// a convenience function to bind each part of the buffer as an SSBO
/// ([`RenderStorage::bind_shader_storage`]).
///
/// This will only bind the parts that specified an SSBO binding in [`Layout`].
///
/// # Operations
/// Available operations are:
/// * [`blit part`](RenderStorage::blit_part) to copy data from the CPU over
///   the GPU buffers for one part.
/// * [`blit section`](RenderStorage::blit_section) to copy data from the CPU
///   over the GPU buffers for a whole section. This takes in raw bytes for
///   type-erasure, as the section may contain parts of varying types.
/// * [`view section`](RenderStorage::view_section) to gain an immutable view
///   of a whole section from the GPU buffers.
/// * [`view part`](RenderStorage::view_part) to gain an immutable view of a
///   part of a section from the GPU buffers.
/// * [`view section mutable`](RenderStorage::view_section_mut) to gain a
///   mutable view of a whole section from the GPU buffers.
/// * [`view part mutable`](RenderStorage::view_part_mut) to gain a mutable
///   view of a part of a section from the GPU buffers.
///
/// The operations related to 'part' are all unsafe, as it isn't possible to
/// verify that the type in the given data corresponds to the same type of the
/// data present on the GPU buffers.
///
/// # Synchronisation
/// [`RenderStorage`] can operate over cross-boundary synchronisation
/// coordination of [`Boundary`] and [`Cross`] over its
/// [`Producer`]-to-[`Consumer`] model.
///
/// [`Boundary`]: crate::state::cross::Boundary
/// [`Cross`]: crate::state::cross::Cross
/// [`Producer`]: crate::state::cross::Producer
/// [`Consumer`]: crate::state::cross::Consumer
#[derive(Clone, Default, Debug)]
pub struct RenderStorage<const PARTS: usize> {
    gl_obj: u32,
    layout: Layout<PARTS>,
    ptr: *mut u8,
}

unsafe impl<const PARTS: usize> Sync for RenderStorage<PARTS> {}
unsafe impl<const PARTS: usize> Send for RenderStorage<PARTS> {}

impl<const PARTS: usize> RenderStorage<PARTS> {
    pub fn new(layout: Layout<PARTS>) -> Self {
        let mut gl_obj = 0;
        let section_length = layout.len();
        let total_length = (section_length * 3) as isize;

        let ptr = unsafe {
            gl::GenBuffers(1, &mut gl_obj);
            gl::BindBuffer(gl::COPY_WRITE_BUFFER, gl_obj);

            let flags = gl::MAP_WRITE_BIT | gl::MAP_COHERENT_BIT | gl::MAP_PERSISTENT_BIT;
            gl::BufferStorage(
                gl::COPY_WRITE_BUFFER,
                total_length,
                ptr::null(),
                flags | gl::DYNAMIC_STORAGE_BIT,
            );

            gl::MapBufferRange(gl::COPY_WRITE_BUFFER, 0, total_length, flags)
        } as *mut u8;

        Self {
            gl_obj,
            layout,
            ptr,
        }
    }

    pub fn layout(&self) -> &Layout<PARTS> {
        &self.layout
    }

    /// Binds all the buffered data of `section` to the GPU's SSBOs.
    ///
    /// Each part is bound to a different SSBO.
    /// The SSBOs binding indices correspond to the order of each part
    /// specified in the buffer's [`layout`](Layout).
    pub fn bind_shader_storage(&self, section: usize) {
        let base_offset = (self.layout.len() * section) as isize;

        for part in 0..PARTS {
            let binding = self.layout.shader[part];
            if binding != u32::MAX {
                let offset = self.layout.offset_at(part) as isize;
                let length = self.layout.length_at(part) as isize;
                unsafe {
                    gl::BindBufferRange(
                        gl::SHADER_STORAGE_BUFFER,
                        binding,
                        self.gl_obj,
                        base_offset + offset,
                        length,
                    );
                }
            }
        }
    }

    /// Copy the given `data` in a `section` of the storage buffer.
    ///
    /// The `section` represents one of the three triple buffer's sections.
    ///
    /// Also see [RenderStorage::blit_part].
    ///
    /// # Panic
    /// * If `section` is not a value within the range (0, 2).
    pub fn blit_section(&self, section: usize, data: &[u8]) {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );

        let src = data.as_ptr();
        let len = self.layout.len();
        let offset = section * len;
        unsafe {
            std::ptr::copy_nonoverlapping(src, self.ptr.add(offset), len);
        }
    }

    /// Get an immutable view to a `section` of the triple buffer.
    ///
    /// The `section` represents one of the three triple buffer's sections.
    ///
    /// Also see [RenderStorage::view_part].
    ///
    /// # Return
    /// Returns a slice of bytes of the given section.
    /// The returned slice is in bytes, a it may contain other sub-sections of
    /// varying types.
    ///
    /// # Panic
    /// The function will panic if `section` is not a value within the range
    /// (0, 2).
    pub fn view_section(&self, section: usize) -> &[u8] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );

        let len = self.layout.len();
        let offset = section * len;
        unsafe { std::slice::from_raw_parts(self.ptr.add(offset), len) }
    }

    pub unsafe fn view_section_raw(&self, section: usize) -> (*mut u8, usize) {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );

        let len = self.layout.len();
        let offset = section * len;

        let ptr = unsafe { self.ptr.add(offset) };
        (ptr, len)
    }

    /// Get a mutable view to a `section` of the triple buffer.
    ///
    /// The `section` represents one of the three triple buffer's sections.
    ///
    /// Also see [RenderStorage::view_part_mut].
    ///
    /// # Return
    /// Returns a slice of bytes of the given section.
    /// The returned slice is in bytes, a it may contain other sub-sections of
    /// varying types.
    ///
    /// # Panic
    /// The function will panic if `section` is not a value within the range
    /// (0, 2).
    pub fn view_section_mut(&self, section: usize) -> &mut [u8] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );

        let len = self.layout.len();
        let offset = section * len;
        unsafe { std::slice::from_raw_parts_mut(self.ptr.add(offset), len) }
    }

    /// Get an immutable view to the `part` of a `section` of the triple
    /// buffer.
    ///
    /// A `part` represents a contiguous stream of data of the same type.
    ///
    /// # Return
    /// An immutable slice of the part of a section of the buffer, casted to
    /// the `T` type parameter of the function.
    ///
    /// # Safety
    /// The type parameter `T` cannot be verified to be the actual type of the
    /// data in this part, the caller must ensure this is always the case.
    ///
    ///  # Panic
    /// * If `section` is not a value within the range (0, 2).
    /// * If `part` is not a valid section, i.e. it is greater than the `PARTS`
    ///   constant type parameter.
    pub unsafe fn view_part<T: Sized>(&self, section: usize, part: usize) -> &[T] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part) / size_of::<T>();

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *const T;
            std::slice::from_raw_parts(ptr, length)
        }
    }

    pub unsafe fn view_part_raw<T: Sized>(&self, section: usize, part: usize) -> (*mut T, usize) {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part) / size_of::<T>();

        let ptr = unsafe { self.ptr.add(base_offset + offset) as *mut T };
        (ptr, length)
    }

    /// Get a mutable view to the `part` of a `section` of the triple buffer.
    ///
    /// A `part` represents a contiguous stream of data of the same type.
    ///
    /// # Return
    /// A mutable slice of the part of a section of the buffer, casted to the
    /// `T` type parameter of the function.
    ///
    /// # Safety
    /// The type parameter `T` cannot be verified to be the actual type of the
    /// data in this part, the caller must ensure this is always the case.
    ///
    /// # Panic
    /// * If `section` is not a value within the range (0, 2).
    /// * If `part` is not a valid section, i.e. it is greater than the `PARTS`
    ///   constant type parameter.
    pub unsafe fn view_part_mut<T: Sized>(&self, section: usize, part: usize) -> &mut [T] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part) / size_of::<T>();

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *mut T;
            std::slice::from_raw_parts_mut(ptr, length)
        }
    }

    /// Copy the given `data` in a `part` of a `section` of the storage buffer.
    ///
    /// A `part` represents a contiguous stream of data of the same type.
    ///
    /// # Safety
    /// The type parameter `T` cannot be verified to be the actual type of the
    /// data in this part, the caller must ensure this is always the case.
    ///
    /// # Panic
    /// * If `section` is not a value within the range (0, 2).
    /// * If `part` is not a valid section, i.e. it is greater than the `PARTS`
    ///   constant type parameter.
    pub unsafe fn blit_part<T: Sized>(&self, section: usize, part: usize, data: &[T]) {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let src = data.as_ptr();
        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        unsafe {
            let dst = self.ptr.add(base_offset + offset) as *mut T;
            std::ptr::copy_nonoverlapping(src, dst, data.len());
        }
    }
}

/// Convenience macro to create a [`Layout`] with an useful enum to access its
/// parts.
///
/// # Example
/// ```
/// layout_buffer! {
///     const Test = 3, {
///         numbers => 0, type u32 = 128;
///         healths => 1, type f32 = 128, shader 1;
///     }
/// };
/// ```
///
/// This will create an enum called `LayoutTest`. Each entry of this enum
/// corresponds to the parts defined in the layout (`LayoutTest::Numbers` and
/// `LayoutTest::Healths`).
///
/// The created enum also contains an associated function `LayoutTest::create`,
/// which will create a [`Layout`] with the defined parts.
///
/// The created enum has the `#[repr(usize)]` attribute, which means that the
/// entries of the enum may be used in [`RenderStorage`] part methods:
///
/// ```
/// layout_buffer! {
///     const Test = 3, {
///         numbers => 0, type u32 = 128;
///         healths => 1, type f32 = 128, shader 1;
///     }
/// };
///
/// let storage = RenderStorage::<2>::new(LayoutTest::create());
/// // the section of the triple buffer, hard-coded to 0 for the example
/// let section_index = 0;
///
/// // SAFETY: as we are using the layout macro's enum of this storage's
/// // layout to index the part, the type of the data contained within the
/// // part is guaranteed to be the f32 type we specified in the macro for this
/// // part.
/// let healths = unsafe {
///     storage.view_part::<f32>(section_index, LayoutTest::Healths as usize)
/// };
/// ```
#[macro_export]
macro_rules! layout_buffer {
    (
        const $name:ty = $len:expr, {
            $(
                $part:ident => $part_idx:expr, type $part_ty:ty = $part_len:expr $(, shader $part_ssbo:expr)? ;
            )+
        }
    ) => {
        paste::paste! {
            #[repr(usize)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
            pub enum [< Layout$name >] {
                $([< $part:camel >] = [< $part_idx _usize>],)+
            }

            impl [< Layout$name >] {
                pub fn create() -> crate::render::data::Layout<$len> {
                    let mut layout = crate::render::data::Layout::<$len>::new();
                    $(
                        layout = layout.section::<$part_ty>($part_len);
                        $(
                            layout = layout.with_shader_storage($part_ssbo);
                        )?
                    )+
                    layout
                }
            }
        }
    };
}

impl<const PARTS: usize> Drop for RenderStorage<PARTS> {
    fn drop(&mut self) {
        unsafe {
            gl::BindBuffer(gl::COPY_WRITE_BUFFER, self.gl_obj);
            gl::UnmapBuffer(gl::COPY_WRITE_BUFFER);
            gl::DeleteBuffers(1, &self.gl_obj);
        }
        self.ptr = std::ptr::null_mut();
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

#[derive(Default, Debug, Clone)]
pub struct SyncBarrier {
    fences: [Option<*const __GLsync>; 3],
}

#[derive(Default, Debug)]
pub struct SyncState {
    locks: AtomicU8,
}

impl SyncBarrier {
    pub fn new() -> Self {
        Self {
            fences: [Option::None; 3],
        }
    }

    pub fn fetch(&mut self, to: &SyncState) {
        let mut bits = 0u8;
        for i in 0..3 {
            if let Some(fence) = self.fences[i].take() {
                let fence_query = unsafe { gl::ClientWaitSync(fence, 0, 0) };
                if fence_query == gl::CONDITION_SATISFIED || fence_query == gl::ALREADY_SIGNALED {
                    unsafe {
                        gl::DeleteSync(fence);
                    }
                } else {
                    match i {
                        0 => bits |= StorageSection::Front as u8,
                        1 => bits |= StorageSection::Back as u8,
                        2 => bits |= StorageSection::Spare as u8,
                        _ => unreachable!(),
                    }
                    self.fences[i] = Some(fence);
                }
            }
        }
        to.set(bits);
    }

    pub fn set(&mut self, index: usize, fence: *const __GLsync) {
        self.fences[index] = Some(fence);
    }
}

impl Drop for SyncBarrier {
    fn drop(&mut self) {
        self.fences
            .into_iter()
            .filter_map(|maybe_fence| maybe_fence)
            .for_each(|fence| unsafe {
                gl::DeleteSync(fence);
            });
    }
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            locks: AtomicU8::new(0),
        }
    }

    /// Performs an `OR` operation on the internal lock bit.
    fn lock_bits(&self, section: u8) {
        self.locks.fetch_or(section, Ordering::Release);
    }

    /// Performs an `AND` operation on the internal lock bit with the inverted
    /// `section` bits.
    fn unlock_bits(&self, section: u8) {
        self.locks.fetch_and(!section, Ordering::Release);
    }

    /// Performs an `OR` operation on the internal lock bit.
    fn lock(&self, section: StorageSection) {
        self.lock_bits(section as u8);
    }

    /// Performs an `AND` operation on the internal lock bit with the inverted
    /// `section` bit.
    fn unlock(&self, section: StorageSection) {
        self.unlock_bits(section as u8);
    }

    fn set(&self, bits: u8) {
        self.locks.store(bits, Ordering::Release);
    }

    pub fn has_lock(&self, section: StorageSection) -> bool {
        let bit = section as u8;
        self.locks.load(Ordering::Acquire) & bit == bit
    }
}

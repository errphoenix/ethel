use std::ptr;

use glam::usize;
use janus::gl;

pub trait GlPropertyEnum {
    fn as_gl_enum(&self) -> u32;
}

pub struct Layout<const PARTS: usize> {
    head: usize,
    last: usize,
    offsets: [usize; PARTS],
    lengths: [usize; PARTS],
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

        self.last += length + offset;
        self.head += 1;
        self
    }

    pub fn offset_at(&self, index: usize) -> usize {
        self.offsets[index]
    }

    pub fn length_at(&self, index: usize) -> usize {
        self.lengths[index]
    }

    pub fn len(&self) -> usize {
        self.last
    }
}

pub struct RenderStorage<const PARTS: usize> {
    gl_obj: u32,
    layout: Layout<PARTS>,
    ptr: *mut u8,
}

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

    /// Binds all the buffered data of `section` to the GPU's SSBOs.
    ///
    /// Each part is bound to a different SSBO.
    /// The SSBOs binding indices correspond to the order of each part
    /// specified in the buffer's [`layout`](Layout).
    pub fn bind_shader_storage(&self, section: usize) {
        let base_offset = (self.layout.len() * section) as isize;

        for part in 0..PARTS {
            let offset = self.layout.offset_at(part) as isize;
            let length = self.layout.length_at(part) as isize;
            unsafe {
                gl::BindBufferRange(
                    gl::SHADER_STORAGE_BUFFER,
                    part as u32,
                    self.gl_obj,
                    base_offset + offset,
                    length,
                );
            }
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
            "render storage is triple buffered, section {section} cannot exist"
        );

        let len = self.layout.len();
        let offset = section * len;
        unsafe { std::slice::from_raw_parts(self.ptr.add(offset), len) }
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
    pub fn view_section_mut(&mut self, section: usize) -> &mut [u8] {
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
    /// # Panic
    /// * If `section` is not a value within the range (0, 2).
    /// * If `part` is not a valid section, i.e. it is greater than the `PARTS`
    ///   constant type parameter.
    ///
    /// # Safety
    /// As the type parameter `T` cannot be verified to be the actual type of
    /// the data in the part, this function is not safe.
    pub unsafe fn view_part<T: Sized>(&self, section: usize, part: usize) -> &[T] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} part"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part);

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *const T;
            std::slice::from_raw_parts(ptr, length)
        }
    }

    /// Get a mutable view to the `part` of a `section` of the triple buffer.
    ///
    /// A `part` represents a contiguous stream of data of the same type.
    ///
    /// # Return
    /// A mutable slice of the part of a section of the buffer, casted to the
    /// `T` type parameter of the function.
    ///
    /// # Panic
    /// * If `section` is not a value within the range (0, 2).
    /// * If `part` is not a valid section, i.e. it is greater than the `PARTS`
    ///   constant type parameter.
    ///
    /// # Safety
    /// As the type parameter `T` cannot be verified to be the actual type of
    /// the data in the part, this function is not safe.
    pub unsafe fn view_part_mut<T: Sized>(&mut self, section: usize, part: usize) -> &mut [T] {
        assert!(
            section < 3,
            "render storage is a triple buffer, section {section} cannot exist"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} part"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part);

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *mut T;
            std::slice::from_raw_parts_mut(ptr, length)
        }
    }
}

#[macro_export]
macro_rules! layout_buffer {
    (
        const $name:ty = $len:expr, {
            $(
                $part:ident => $part_idx:expr, type $part_ty:ty = $part_len:expr;
            )+
        }
    ) => {
        paste::paste! {
            #[repr(usize)]
            pub enum [< Layout$name >] {
                $([< $part:camel >] = [< $part_idx _usize>],)+
            }

            impl [< Layout$name >] {
                pub fn create() -> crate::render::data::Layout<$len> {
                    let mut layout = crate::render::data::Layout::<$len>::new();
                    $(
                        layout = layout.section::<$part_ty>($part_len);
                    )+
                    layout
                }
            }
        }
    };
}

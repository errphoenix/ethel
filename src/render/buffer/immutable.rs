use std::rc::Rc;

use crate::render::buffer::Layout;

pub fn uninit<const PARTS: usize>(layout: Layout<PARTS>) -> UninitImmutableBuffer<PARTS> {
    UninitImmutableBuffer::new(layout)
}

#[derive(Debug, Default)]
pub struct UninitImmutableBuffer<const PARTS: usize> {
    gl_obj: u32,
    ptr: *mut u8,
    layout: Layout<PARTS>,

    // Unitialised buffer must not be sent to other threads
    // Drop impl requires GL calls, as does its creation
    _marker: std::marker::PhantomData<Rc<()>>,
}

impl<const PARTS: usize> UninitImmutableBuffer<PARTS> {
    pub fn new(layout: Layout<PARTS>) -> Self {
        let mut gl_obj = 0;
        let total_length = layout.len() as isize;

        let ptr = unsafe {
            janus::gl::CreateBuffers(1, &mut gl_obj);
            janus::gl::NamedBufferStorage(
                gl_obj,
                total_length,
                std::ptr::null(),
                janus::gl::MAP_WRITE_BIT,
            );
            janus::gl::MapNamedBufferRange(
                gl_obj,
                0,
                total_length,
                janus::gl::MAP_WRITE_BIT | janus::gl::MAP_INVALIDATE_BUFFER_BIT,
            )
        } as *mut u8;

        Self {
            layout,
            ptr,
            gl_obj,
            _marker: std::marker::PhantomData,
        }
    }

    /// Fill the `partition` of the buffer with the given `data`.
    ///
    /// # Panics
    /// * If `partition` is greater or equal to `PARTS`, i.e. it is not a
    ///   valid partition.
    /// * If the length of the given `data` is greater than the length
    ///   allocated for the specified `partition` in the buffer's [`Layout`].
    ///
    /// # Safety
    /// This operation does not ensure that the type `T` of `data` matches the
    /// type and alignment of the buffer's [`Layout`] specification.
    ///
    /// Passing the wrong type `T` might lead to undefined behaviour, and will
    /// cause VRAM corruption.
    pub fn fill_partition<T: Sized>(&mut self, partition: usize, data: &[T]) {
        assert!(
            partition < PARTS,
            "attempted to fill partition {partition} of a buffer that contains only {PARTS} partitions"
        );

        let length = self.layout.length_at(partition);
        let len_bytes = data.len() * size_of::<T>();
        assert!(
            length >= len_bytes,
            "length of data cannot fit in the allocated block of this partition"
        );

        let offset = self.layout.offset_at(partition);

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr() as *const u8,
                self.ptr.add(offset),
                len_bytes,
            );
        }
    }

    /// Unmap the buffer and forbid any further changes to its contents.
    ///
    /// # Returns
    /// An [`ImmutableBuffer`] preserving the OpenGL buffer object.
    pub fn finish(self) -> ImmutableBuffer<PARTS> {
        unsafe {
            janus::gl::UnmapNamedBuffer(self.gl_obj);
        }

        ImmutableBuffer {
            gl_obj: self.gl_obj,
            layout: self.layout.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<const PARTS: usize> Drop for UninitImmutableBuffer<PARTS> {
    fn drop(&mut self) {
        unsafe {
            janus::gl::UnmapNamedBuffer(self.gl_obj);
            janus::gl::DeleteBuffers(1, &self.gl_obj);
        }
        self.ptr = std::ptr::null_mut();
    }
}

#[derive(Debug, Default)]
pub struct ImmutableBuffer<const PARTS: usize> {
    gl_obj: u32,
    layout: Layout<PARTS>,

    // Immutable buffer must not be sent to other threads
    // All operations related to immutable buffers require GL calls, the logic
    // thread has no business with it
    _marker: std::marker::PhantomData<Rc<()>>,
}

impl<const PARTS: usize> ImmutableBuffer<PARTS> {
    pub fn bind_shader_storage(&self) {
        for part in 0..PARTS {
            if let Some(binding) = self.layout.ssbo_of(part) {
                let offset = self.layout.offset_at(part) as isize;
                let length = self.layout.length_at(part) as isize;
                unsafe {
                    janus::gl::BindBufferRange(
                        janus::gl::SHADER_STORAGE_BUFFER,
                        binding,
                        self.gl_obj,
                        offset,
                        length,
                    );
                }
            }
        }
    }
}

impl<const PARTS: usize> Drop for ImmutableBuffer<PARTS> {
    fn drop(&mut self) {
        unsafe {
            janus::gl::DeleteBuffers(1, &self.gl_obj);
        }
    }
}

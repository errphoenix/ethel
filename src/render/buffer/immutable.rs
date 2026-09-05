use std::rc::Rc;

use crate::render::buffer::Layout;

macro_rules! assert_partition {
    ($pt:expr, $pi:expr) => {
        let pt = $pt;
        let pi = $pi;
        assert!(
            pi < pt,
            "attempted to access partition {pi} in a buffer with only {pt} partitions"
        );
    };
}

pub fn uninit<const PARTS: usize>(layout: Layout<PARTS>) -> UninitImmutableBuffer<PARTS> {
    UninitImmutableBuffer::new(layout)
}

#[derive(Debug, Default)]
pub struct UninitImmutableBuffer<const PARTS: usize> {
    gl_obj: u32,
    ptr: *mut u8,
    layout: Layout<PARTS>,
    mapped: bool,

    // Unitialised buffer must not be sent to other threads
    // Drop impl requires GL calls, as does its creation
    _marker: std::marker::PhantomData<Rc<()>>,
}
impl<const PARTS: usize> janus::GpuResource for UninitImmutableBuffer<PARTS> {
    fn resource_id(&self) -> u32 {
        self.gl_obj
    }
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
                janus::gl::MAP_WRITE_BIT | janus::gl::MAP_READ_BIT,
            );
            janus::gl::ClearNamedBufferData(
                gl_obj,
                janus::gl::R32UI,
                janus::gl::RED_INTEGER,
                janus::gl::UNSIGNED_INT,
                0 as *const _,
            );
            janus::gl::MapNamedBufferRange(
                gl_obj,
                0,
                total_length,
                janus::gl::MAP_WRITE_BIT | janus::gl::MAP_READ_BIT,
            )
        } as *mut u8;

        Self {
            layout,
            ptr,
            gl_obj,
            mapped: true,
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
    pub unsafe fn fill_partition<T: Sized>(&mut self, partition: usize, data: &[T]) {
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
    pub fn finish(mut self) -> ImmutableBuffer<PARTS> {
        self.mapped = false;

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
        if self.mapped {
            unsafe {
                janus::gl::UnmapNamedBuffer(self.gl_obj);
                janus::gl::DeleteBuffers(1, &self.gl_obj);
            }
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
impl<const PARTS: usize> janus::GpuResource for ImmutableBuffer<PARTS> {
    fn resource_id(&self) -> u32 {
        self.gl_obj
    }
}
impl<const PARTS: usize> ImmutableBuffer<PARTS> {
    /// Binds a single partition of buffered data to the GPU's SSBOs.
    ///
    /// The data will be bound to the SSBO specified by the given index
    /// `ssbo_index` if provided. Otherwise, the SSBO binding index will
    /// correspond to the one specified in this buffer's [`Layout`].
    ///
    /// This only binds one partition as a GLSL runtime array.
    ///
    /// # Panic
    /// * If `partition` does not correspond to a valid partition index.
    /// * If `ssbo_index` is `None` and the buffer's layout does not specify
    ///   an ssbo index for the specified `partition` to fallback to.
    pub fn bind_shader_storage_single(&self, partition: usize, ssbo_index: Option<u32>) {
        assert_partition!(PARTS, partition);

        let binding = ssbo_index
            .or_else(|| self.layout.ssbo_of(partition))
            .expect("no ssbo index provided and missing fallback");

        let offset = self.layout.offset_at(partition) as isize;
        let length = self.layout.length_at(partition) as isize;
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

    /// Binds `partition_len` contiguous partitions starting from
    /// `partition_base` as a single SSBO.
    ///
    /// All partitions are bound to `ssbo_index` under a single SSBO using
    /// static GLSL arrays. The GLSL SSBO array length must match the capacity
    /// of each partition here.
    ///
    /// If `ssbo_index` is provided, it will be used as the SSBO binding index;
    /// else, `partition_base`'s binding index is used if any is defined.
    ///
    /// # Panic
    /// * If `partition` does not correspond to a valid partition index.
    /// * If `ssbo_index` is `None` and the buffer's layout does not specify
    ///   an ssbo index for the specified `partition` to fallback to.
    pub fn bind_shader_storage_arrays(
        &self,
        partition_base: usize,
        partition_len: usize,
        ssbo_index: Option<u32>,
    ) {
        assert_partition!(PARTS, partition_base + partition_len - 1);

        let ssbo_index = ssbo_index
            .or_else(|| self.layout.ssbo_of(partition_base))
            .expect("no ssbo index provided and missing fallback");

        let base_offset = self.layout.offset_at(partition_base) as isize;
        let tot_length = {
            let mut i = 0;
            let mut length = 0;
            while i < partition_len {
                length += self.layout.length_at(i + partition_base);
                i += 1;
            }
            length as isize
        };

        unsafe {
            janus::gl::BindBufferRange(
                janus::gl::SHADER_STORAGE_BUFFER,
                ssbo_index,
                self.gl_obj,
                base_offset,
                tot_length,
            );
        }
    }

    /// Binds all the buffered data to the GPU's SSBOs.
    ///
    /// Each partition is bound to a different SSBO.
    /// The SSBOs binding indices correspond to the one specified in this
    /// buffer's [`layout`](Layout).
    ///
    /// This binds each partition as a separate GLSL runtime array.
    pub fn bind_shader_storage(&self) {
        for part in 0..PARTS {
            if self.layout.ssbo_of(part).is_some() {
                self.bind_shader_storage_single(part, None);
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

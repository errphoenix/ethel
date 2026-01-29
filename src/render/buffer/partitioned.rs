use crate::render::buffer::{InitStrategy, View, ViewMut, layout::Layout};

/// A partitioned triple buffered OpenGL buffer over a single memory block.
///
/// This handles alignments and offsets of each memory section and part (a
/// contiguous memory block of data of the same type).
///
/// # OpenGL Representation
/// The GPU buffers are coherent persistent copy-write buffers. It includes
/// a convenience function to bind each part of the buffer as an SSBO
/// ([`PartitionedTriBuffer::bind_shader_storage`]).
///
/// This will only bind the parts that specified an SSBO binding in [`Layout`].
///
/// # Operations
/// Available operations are:
/// * [`blit part`](PartitionedTriBuffer::blit_part) to copy data from the CPU over
///   the GPU buffers for one part.
/// * [`blit section`](PartitionedTriBuffer::blit_section) to copy data from the CPU
///   over the GPU buffers for a whole section. This takes in raw bytes for
///   type-erasure, as the section may contain parts of varying types.
/// * [`view section`](PartitionedTriBuffer::view_section) to gain an immutable view
///   of a whole section from the GPU buffers.
/// * [`view part`](PartitionedTriBuffer::view_part) to gain an immutable view of a
///   part of a section from the GPU buffers.
/// * [`view section mutable`](PartitionedTriBuffer::view_section_mut) to gain a
///   mutable view of a whole section from the GPU buffers.
/// * [`view part mutable`](PartitionedTriBuffer::view_part_mut) to gain a mutable
///   view of a part of a section from the GPU buffers.
///
/// <div class="warning">
///
/// ### Note
///
/// Similarly to [`TriBuffer`], reading from the GPU buffers is slower than
/// reading from system memory, thus it is not recommended to mutate data
/// through the `view_*_mut` functions in performance-critical scenarios.
///
/// Prefer the usage of `blit_part` and `blit_section` to mutate data as these
/// correspond to a single `memcpy` operation directly to the underlying
/// memory, which is significantly faster because the required modification is
/// reduced to a single operation.
/// They're also not unsafe, unlike `view_*_mut`.
///
/// </div>
///
/// The operations related to 'part' are all unsafe, as it isn't possible to
/// verify that the type in the given data corresponds to the same type of the
/// data present on the GPU buffers.
///
/// # Synchronisation
/// [`PartitionedTriBuffer`] can operate over cross-boundary synchronisation
/// coordination of [`Boundary`] and [`Cross`] over its
/// [`Producer`]-to-[`Consumer`] model.
///
/// [`Boundary`]: crate::state::cross::Boundary
/// [`Cross`]: crate::state::cross::Cross
/// [`Producer`]: crate::state::cross::Producer
/// [`Consumer`]: crate::state::cross::Consumer
#[derive(Clone, Default, Debug)]
pub struct PartitionedTriBuffer<const PARTS: usize> {
    gl_obj: u32,
    layout: Layout<PARTS>,
    ptr: *mut u8,
}

unsafe impl<const PARTS: usize> Sync for PartitionedTriBuffer<PARTS> {}
unsafe impl<const PARTS: usize> Send for PartitionedTriBuffer<PARTS> {}

impl<const PARTS: usize> PartitionedTriBuffer<PARTS> {
    pub fn new(layout: Layout<PARTS>) -> Self {
        let mut gl_obj = 0;
        let section_length = layout.len();
        let total_length = (section_length * 3) as isize;

        let ptr = unsafe {
            janus::gl::GenBuffers(1, &mut gl_obj);
            janus::gl::BindBuffer(janus::gl::COPY_WRITE_BUFFER, gl_obj);

            let flags = janus::gl::MAP_WRITE_BIT
                | janus::gl::MAP_COHERENT_BIT
                | janus::gl::MAP_PERSISTENT_BIT;
            janus::gl::BufferStorage(
                janus::gl::COPY_WRITE_BUFFER,
                total_length,
                std::ptr::null(),
                flags | janus::gl::DYNAMIC_STORAGE_BIT,
            );

            janus::gl::MapBufferRange(janus::gl::COPY_WRITE_BUFFER, 0, total_length, flags)
        } as *mut u8;

        Self {
            gl_obj,
            layout,
            ptr,
        }
    }

    pub fn initialise_part<T: Sized + Clone, F: Fn() -> T>(
        &self,
        part: usize,
        strategy: InitStrategy<T, F>,
    ) {
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let len = self.layout.length_at(part);
        let offset = self.layout.offset_at(part);

        match strategy {
            InitStrategy::Zero => {
                for i in 0..3 {
                    let section_offset = (self.layout.len() * i) as isize;
                    unsafe {
                        janus::gl::ClearNamedBufferSubData(
                            self.gl_obj,
                            janus::gl::R32UI,
                            section_offset + offset as isize,
                            len as isize,
                            janus::gl::RED_INTEGER,
                            janus::gl::UNSIGNED_INT,
                            std::ptr::null(),
                        );
                    }
                }
            }
            InitStrategy::FillWith(func) => {
                let ptr = self.ptr as *mut T;
                let len = len / size_of::<T>();

                for i in 0..3 {
                    unsafe {
                        let ptr = ptr.add(self.layout.len() * i);
                        for i in 0..len {
                            std::ptr::write(ptr.add(i), func());
                        }
                    }
                }
            }
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
    ///
    /// # Panic
    /// If `section` is not a value within the range (0, 2).
    pub fn bind_shader_storage(&self, section: usize) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let base_offset = (self.layout.len() * section) as isize;
        for part in 0..PARTS {
            if let Some(binding) = self.layout.ssbo_of(part) {
                let offset = self.layout.offset_at(part) as isize;
                let length = self.layout.length_at(part) as isize;
                unsafe {
                    janus::gl::BindBufferRange(
                        janus::gl::SHADER_STORAGE_BUFFER,
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
    /// Also see [PartitionedTriBuffer::blit_part].
    ///
    /// # Panic
    /// If `section` is not a value within the range (0, 2).
    pub fn blit_section(&self, section: usize, data: &[u8]) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let src = data.as_ptr();
        let section_len = self.layout.len();
        let data_len = section_len.min(data.len());
        let offset = section * section_len;
        unsafe {
            std::ptr::copy_nonoverlapping(src, self.ptr.add(offset), data_len);
        }
    }

    /// Get an immutable view to a `section` of the triple buffer.
    ///
    /// The `section` represents one of the three triple buffer's sections.
    ///
    /// Also see [PartitionedTriBuffer::view_part].
    ///
    /// # Return
    /// Returns a slice of bytes of the given section.
    /// The returned slice is in bytes, a it may contain other sub-sections of
    /// varying types.
    ///
    /// # Panic
    /// The function will panic if `section` is not a value within the range
    /// (0, 2).
    pub fn view_section(&self, section: usize) -> View<'_, u8> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let length = self.layout.len();
        let offset = section * length;
        unsafe {
            let slice = std::slice::from_raw_parts(self.ptr.add(offset), length);
            View {
                slice,
                offset: offset as u32,
                length: length as u32,
                source: self.gl_obj,
            }
        }
    }

    pub unsafe fn view_section_raw(&self, section: usize) -> (*mut u8, usize) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
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
    /// Also see [PartitionedTriBuffer::view_part_mut].
    ///
    /// # Return
    /// Returns a slice of bytes of the given section.
    /// The returned slice is in bytes, a it may contain other sub-sections of
    /// varying types.
    ///
    /// # Panic
    /// The function will panic if `section` is not a value within the range
    /// (0, 2).
    pub fn view_section_mut(&self, section: usize) -> ViewMut<'_, u8> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );

        let length = self.layout.len();
        let offset = section * length;
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr.add(offset), length);
            ViewMut {
                slice,
                offset: offset as u32,
                length: length as u32,
                source: self.gl_obj,
            }
        }
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
    pub unsafe fn view_part<T: Sized>(&self, section: usize, part: usize) -> View<'_, T> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part);
        let len = length / size_of::<T>();

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *const T;
            let slice = std::slice::from_raw_parts(ptr, len);
            View {
                slice,
                offset: offset as u32,
                length: len as u32,
                source: self.gl_obj,
            }
        }
    }

    pub unsafe fn view_part_raw<T: Sized>(&self, section: usize, part: usize) -> (*mut T, usize) {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
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
    pub unsafe fn view_part_mut<T: Sized>(&self, section: usize, part: usize) -> ViewMut<'_, T> {
        assert!(
            section < 3,
            "attempted to access section {section} in a triple buffer (3 sections)"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let length = self.layout.length_at(part);
        let len = length / size_of::<T>();

        unsafe {
            let ptr = self.ptr.add(base_offset + offset) as *mut T;
            let slice = std::slice::from_raw_parts_mut(ptr, len);
            ViewMut {
                slice,
                offset: offset as u32,
                length: length as u32,
                source: self.gl_obj,
            }
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
            "attempted to access section {section} in a triple buffer (3 sections)"
        );
        assert!(
            part < PARTS,
            "attempted to access part {part}, but the buffer only has {PARTS} parts"
        );

        let src = data.as_ptr();
        let base_offset = section * self.layout.len();
        let offset = self.layout.offset_at(part);
        let data_len = self.layout.length_at(part).min(data.len());

        unsafe {
            let dst = self.ptr.add(base_offset + offset) as *mut T;
            std::ptr::copy_nonoverlapping(src, dst, data_len);
        }
    }
}

impl<const PARTS: usize> Drop for PartitionedTriBuffer<PARTS> {
    fn drop(&mut self) {
        unsafe {
            janus::gl::BindBuffer(janus::gl::COPY_WRITE_BUFFER, self.gl_obj);
            janus::gl::UnmapBuffer(janus::gl::COPY_WRITE_BUFFER);
            janus::gl::DeleteBuffers(1, &self.gl_obj);
        }
        self.ptr = std::ptr::null_mut();
    }
}

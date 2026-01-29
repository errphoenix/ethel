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

    pub fn partition<T: Sized>(mut self, count: usize) -> Self {
        let head = self.head;
        assert!(head < PARTS, "layout only permits {PARTS} partitions");
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

    /// The local offset (in bytes) of the part at `index`.
    pub fn offset_at(&self, index: usize) -> usize {
        self.offsets[index]
    }

    /// The length (in bytes) of the part at `index`.
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

/// Convenience macro to create a [`Layout`] with a useful enum to access
/// buffer partitions.
///
/// # Example
/// ```
/// layout_buffer! {
///     const Test: 3, {
///         enum numbers: 32 => {
///             type u32;
///             bind 0;
///         };
///
///         enum healths: 128 => {
///             type f32;
///             bind 1;
///             init with {
///                 20.0
///             };
///             shader 0;
///         };
///
///         enum positions: 128 {
///             type (f32, f32);
///             bind 2;
///             init with {
///                 get_spawn_pos()
///             };
///             shader 1;
///         };
///     }
/// };
/// ```
///
/// This will create an enum called `LayoutTest`. Each entry of this enum
/// corresponds to the parts defined in the layout (`LayoutTest::Numbers`,
/// `LayoutTest::Healths`, and `LayoutTest::Positions`).
///
/// ## Access
///
/// The created enum also contains an associated function `LayoutTest::create`,
/// which will create a [`Layout`] with the defined parts.
///
/// The created enum has the `#[repr(usize)]` attribute, which means that the
/// entries of the enum may be used in [`PartitionedTriBuffer`]'s view_part*
/// methods:
///
/// ```
/// layout_buffer! {
///     const Test: 3, {
///         enum numbers: 32 => {
///             type u32;
///             bind 0;
///         };
///
///         enum healths: 128 => {
///             type f32;
///             bind 1;
///             init with {
///                 20.0
///             };
///             shader 0;
///         };
///
///         enum positions: 128 {
///             type (f32, f32);
///             bind 2;
///             init with {
///                 get_spawn_pos()
///             };
///             shader 1;
///         };
///     }
/// };
///
/// let storage = PartitionedTriBuffer::<3>::new(LayoutTest::create());
/// // the section of the triple buffer, hard-coded to 0 for the example
/// let section_index = 0;
///
/// // SAFETY: as we are using the layout macro's enum of this buffer's
/// // layout to index the partition, the type of the data contained within the
/// // partition is guaranteed to be the f32 type we specified in the macro
/// // for this partition.
/// let healths = unsafe {
///     storage.view_part::<f32>(section_index, LayoutTest::Healths as usize)
/// };
/// ```
///
/// ## Partitioned Buffer Initialisation
///
/// To properly initialise a [`PartitionedTriBuffer`], the macro generates yet
/// another convenience associated function to ensure the data within each
/// defined partition is not "garbage data".
///
/// This is an associated function of the generated enum, such as
/// `LayoutTest::initialise_partitions`.
///
/// The value of the initialised data is equal for all entries of a partition,
/// and it corresponds to the value returned within the 'init with' code block.
/// If this code block is absent, all bytes present in the partition are reset
/// to 0 upon initialisation.
///
/// These corresponds to the [`InitStrategy::FillWith`] and
/// [`InitStrategy::Zero`] initialisation strategies respectively, with the
/// latter being the default.
///
/// [`InitStrategy::Zero`]: super::InitStrategy::Zero
/// [`InitStrategy::FillWith`]: super::InitStrategy::FillWith
/// [`PartitionedTriBuffer`]: super::partitioned::PartitionedTriBuffer
#[macro_export]
macro_rules! layout_buffer {
    (
        const $name:ty: $len:expr, {
            $(
                enum $part:ident: $part_len:expr => {
                    type $part_ty:ty;
                    bind $part_idx:expr;
                    $(init with $init:block;)?
                    $(shader $part_ssbo:expr;)?
                };
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
                pub fn create() -> crate::render::buffer::layout::Layout<$len> {
                    let mut layout = crate::render::buffer::layout::Layout::<$len>::new();
                    $(
                        layout = layout.partition::<$part_ty>($part_len);
                        $(
                            layout = layout.with_shader_storage($part_ssbo);
                        )?
                    )+
                    layout
                }

                pub fn initialise_partitions<const PARTS: usize>(buffer: &crate::render::buffer::partitioned::PartitionedTriBuffer<PARTS>) {
                    $(
                        #[allow(unused_variables)]
                        {
                            let mode = crate::render::buffer::InitStrategy::<$part_ty, fn() -> $part_ty>::Zero;
                            $(
                                let mode = crate::render::buffer::InitStrategy::FillWith(|| $init);
                            )?
                            buffer.initialise_partition::<$part_ty, _>($part_idx, mode);
                        }
                    )+
                }
            }
        }
    };
}

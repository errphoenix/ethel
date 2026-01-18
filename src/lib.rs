pub mod shader;
pub mod mesh;

pub(crate) mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub trait GlPropertyEnum {
    fn as_gl_enum(&self) -> u32;
}

pub struct RenderBuffer<const BUF_COUNT: usize> {
    vao: u32,
    vbo: [u32; BUF_COUNT],
}

impl<const BUF_COUNT: usize> RenderBuffer<BUF_COUNT> {
    pub fn from_buffers(buffers: [u32; BUF_COUNT]) -> Self {
        let mut vao = 0;
        unsafe {
            gl::CreateVertexArrays(1, &mut vao);
        }
        Self { vao, vbo: buffers }
    }

    pub fn with_buffers(buffers: CreateBuffers) -> Self {
        let mut vao = 0;
        unsafe {
            gl::CreateVertexArrays(1, &mut vao);
        }

        let vbo = {
            let mut vbo = [0; BUF_COUNT];
            for (i, buf) in buffers.create(vao).enumerate().take(BUF_COUNT) {
                vbo[i] = buf;
            }
            vbo
        };

        Self { vao, vbo }
    }

    pub fn alloc_buffer<T>(&self, index: usize, usage: BufferUsage, len: isize, ptr: *const T) {
        unsafe {
            gl::NamedBufferData(self.vbo[index], len, ptr as *const _, usage.as_gl_enum());
        }
    }

    pub fn alloc_buffer_uninit(&self, index: usize, usage: BufferUsage, len: isize) {
        unsafe {
            gl::NamedBufferData(self.vbo[index], len, std::ptr::null(), usage.as_gl_enum());
        }
    }

    pub fn alloc_buffer_slice<T>(&self, index: usize, usage: BufferUsage, bytes: &[T]) {
        unsafe {
            gl::NamedBufferData(
                self.vbo[index],
                bytes.len() as isize,
                bytes.as_ptr() as *const _,
                usage.as_gl_enum(),
            );
        }
    }

    pub fn upload_buffer<T>(&self, index: usize, offset: isize, len: isize, ptr: *const T) {
        unsafe {
            gl::NamedBufferSubData(self.vbo[index], offset, len, ptr as *const _);
        }
    }

    pub fn upload_buffer_slice<T>(&self, index: usize, offset: isize, bytes: &[T]) {
        unsafe {
            gl::NamedBufferSubData(
                self.vbo[index],
                offset,
                bytes.len() as isize,
                bytes.as_ptr() as *const _,
            );
        }
    }
}

impl RenderBuffer<0> {
    pub fn new() -> Self {
        let mut vao = 0;
        unsafe {
            gl::CreateVertexArrays(1, &mut vao);
        }
        Self { vao, vbo: [0; 0] }
    }
}

impl<const BUF_COUNT: usize> Drop for RenderBuffer<BUF_COUNT> {
    fn drop(&mut self) {
        for i in 0..BUF_COUNT {
            unsafe {
                gl::DeleteBuffers(1, &self.vbo[i]);
            }
        }
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum BufferKind {
    #[default]
    Array,
    AtomicCounter,
    CopyRead,
    CopyWrite,
    Dispatch,
    Draw,
    Element,
    PixelPack,
    PixelUnpack,
    Query,
    ShaderStorage,
    Texture,
    TransformFeedback,
    Uniform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum AttributeKind {
    #[default]
    Float,
    Byte,
    Integer,
    IntegerSigned,
    ByteSigned,
}

impl AttributeKind {
    pub fn size_bytes(&self) -> usize {
        match self {
            AttributeKind::Float => 4,
            AttributeKind::Byte => 1,
            AttributeKind::Integer => 4,
            AttributeKind::IntegerSigned => 4,
            AttributeKind::ByteSigned => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum StorageKind {
    #[default]
    Static,
    Dynamic,
    Client,
    Persistent {
        read: bool,
        write: bool,
    },
    Coherent {
        read: bool,
        write: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum BufferUsage {
    StreamDraw,
    StreamRead,
    StreamCopy,

    #[default]
    StaticDraw,
    StaticRead,
    StaticCopy,

    DynamicDraw,
    DynamicRead,
    DynamicCopy,
}

impl GlPropertyEnum for BufferUsage {
    fn as_gl_enum(&self) -> u32 {
        match self {
            BufferUsage::StreamDraw => gl::STREAM_DRAW,
            BufferUsage::StreamRead => gl::STREAM_READ,
            BufferUsage::StreamCopy => gl::STREAM_COPY,
            BufferUsage::StaticDraw => gl::STATIC_DRAW,
            BufferUsage::StaticRead => gl::STATIC_READ,
            BufferUsage::StaticCopy => gl::STATIC_COPY,
            BufferUsage::DynamicDraw => gl::DYNAMIC_DRAW,
            BufferUsage::DynamicRead => gl::DYNAMIC_READ,
            BufferUsage::DynamicCopy => gl::DYNAMIC_COPY,
        }
    }
}

impl GlPropertyEnum for BufferKind {
    fn as_gl_enum(&self) -> u32 {
        match self {
            BufferKind::Array => gl::ARRAY_BUFFER,
            BufferKind::AtomicCounter => gl::ATOMIC_COUNTER_BUFFER,
            BufferKind::CopyRead => gl::COPY_READ_BUFFER,
            BufferKind::CopyWrite => gl::COPY_WRITE_BUFFER,
            BufferKind::Dispatch => gl::DISPATCH_INDIRECT_BUFFER,
            BufferKind::Draw => gl::DRAW_INDIRECT_BUFFER,
            BufferKind::Element => gl::ELEMENT_ARRAY_BUFFER,
            BufferKind::PixelPack => gl::PIXEL_PACK_BUFFER,
            BufferKind::PixelUnpack => gl::PIXEL_UNPACK_BUFFER,
            BufferKind::Query => gl::QUERY_BUFFER,
            BufferKind::ShaderStorage => gl::SHADER_STORAGE_BUFFER,
            BufferKind::Texture => gl::TEXTURE_BUFFER,
            BufferKind::TransformFeedback => gl::TRANSFORM_FEEDBACK_BUFFER,
            BufferKind::Uniform => gl::UNIFORM_BUFFER,
        }
    }
}

impl GlPropertyEnum for AttributeKind {
    fn as_gl_enum(&self) -> u32 {
        match self {
            AttributeKind::Float => gl::FLOAT,
            AttributeKind::Byte => gl::UNSIGNED_BYTE,
            AttributeKind::Integer => gl::UNSIGNED_INT,
            AttributeKind::IntegerSigned => gl::INT,
            AttributeKind::ByteSigned => gl::BYTE,
        }
    }
}

impl GlPropertyEnum for StorageKind {
    fn as_gl_enum(&self) -> u32 {
        match self {
            StorageKind::Dynamic => gl::DYNAMIC_STORAGE_BIT,
            StorageKind::Client => gl::CLIENT_STORAGE_BIT,
            StorageKind::Persistent {
                read: false,
                write: false,
            } => panic!("Persistent storage kind is neither write or read"),
            StorageKind::Coherent {
                read: false,
                write: false,
            } => panic!("Persistent (with coherent) storage kind is neither write or read"),
            StorageKind::Persistent { read, write } => {
                let mut bit = gl::MAP_PERSISTENT_BIT;
                if *read {
                    bit |= gl::MAP_READ_BIT;
                }
                if *write {
                    bit |= gl::MAP_WRITE_BIT;
                }
                bit
            }
            StorageKind::Coherent { read, write } => {
                let mut bit = gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT;
                if *read {
                    bit |= gl::MAP_READ_BIT;
                }
                if *write {
                    bit |= gl::MAP_WRITE_BIT;
                }
                bit
            }
            StorageKind::Static => 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreateBuffer {
    kind: BufferKind,
    attributes: Vec<LayoutBuffer>,
}

impl CreateBuffer {
    fn create(mut self, vaobj: u32, buf_index: u32) -> u32 {
        let vbo = {
            let mut vbo = 0;
            unsafe {
                gl::CreateBuffers(1, &mut vbo);
            }
            vbo
        };

        let stride = self.attributes.iter().fold(0, |s, o| s + o.size_bytes()) as i32;
        let mut offset = 0;
        self.attributes
            .drain(..)
            .enumerate()
            .for_each(|(i, layout)| unsafe {
                let index = i as u32;
                let size = layout.attribute_size as i32;
                let r#type = layout.attribute_kind.as_gl_enum();
                let normalized = layout.normalised as u8;

                gl::VertexArrayAttribFormat(vaobj, index, size, r#type, normalized, offset);
                gl::VertexArrayAttribBinding(vaobj, index, buf_index);
                gl::VertexArrayVertexBuffer(vaobj, buf_index, vbo, 0, stride);
                gl::EnableVertexArrayAttrib(vaobj, index);

                offset += layout.size_bytes();
            });

        vbo
    }
}

#[derive(Debug, Default, Clone)]
pub struct LayoutBuffer {
    attribute_kind: AttributeKind,
    attribute_size: u8,
    normalised: bool,
}

impl LayoutBuffer {
    pub fn size_bytes(&self) -> u32 {
        self.attribute_kind.size_bytes() as u32 * self.attribute_size as u32
    }

    pub fn with_type(mut self, kind: AttributeKind, size: u8) -> Self {
        self.attribute_kind = kind;
        self.attribute_size = size;
        self
    }

    pub fn normalised(mut self, normalised: bool) -> Self {
        self.normalised = normalised;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreateLayout(Vec<LayoutBuffer>);

impl CreateLayout {
    pub fn new(attrib_count: usize) -> Self {
        Self(Vec::with_capacity(attrib_count))
    }

    pub fn attribute<F>(mut self, builder: F) -> Self
    where
        F: Fn(LayoutBuffer) -> LayoutBuffer,
    {
        self.0.push(builder(LayoutBuffer::default()));
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreateBuffers {
    buffers: Vec<CreateBuffer>,
}

impl CreateBuffers {
    pub fn new(buffer_count: usize) -> Self {
        let mut buffers = Vec::with_capacity(buffer_count);
        buffers.push(Default::default());
        Self { buffers }
    }

    pub fn push(mut self) -> Self {
        self.buffers.push(Default::default());
        self
    }

    pub fn kind(mut self, kind: BufferKind) -> Self {
        self.buffers
            .last_mut()
            .expect("no buffer bound during creation")
            .kind = kind;
        self
    }

    pub fn layout(mut self, layout: CreateLayout) -> Self {
        self.buffers
            .last_mut()
            .expect("no buffer bound during creation")
            .attributes = layout.0;
        self
    }

    fn create(self, vaobj: u32) -> impl Iterator<Item = u32> {
        self.buffers
            .into_iter()
            .enumerate()
            .map(move |(i, buf)| buf.create(vaobj, i as u32))
    }
}

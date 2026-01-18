/// The ID that represents a Mesh present on GPU memory, from the CPU.
///
/// It is used to link objects or "renderables" to a mesh that is present on
/// the GPU through its [`Metadata`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Id(pub(crate) u32);

/// The position and length of a Mesh on GPU memory.
///
/// This is usually accessed through a [`Mesh ID`](Id), and it is the only
/// instance-specific mesh information that is passed onto the GPU.
///
/// It indicates the starting index in the vertex buffer and the total length
/// of the mesh, which is used to:
/// * Determine the offset of the next [`Mesh Metadata`](Metadata).
/// * Specify the amount of vertices the GPU has to draw for the instance using
///   the mesh.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Metadata {
    pub(crate) offset: u32,
    pub(crate) length: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Meshadata {
    metadata: Vec<Metadata>,
    head: u32,
}

impl Meshadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.metadata.clear();
        self.head = 0;
    }

    pub fn add(&mut self, length: u32) -> Id {
        let id = self.metadata.len() as u32;
        self.metadata.push(Metadata {
            offset: self.head,
            length,
        });
        self.head += length;
        Id(id)
    }

    pub fn get(&self, id: Id) -> &Metadata {
        &self.metadata[id.0 as usize]
    }

    pub fn head(&self) -> u32 {
        self.head
    }
}

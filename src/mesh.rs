use std::ops::Deref;

use crate::layout_buffer;

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

#[derive(Default, Clone, Debug)]
pub struct Meshadata {
    metadata: Vec<Metadata>,
    head: u32,
}

impl Meshadata {
    pub fn new() -> Self {
        Self {
            metadata: Vec::with_capacity(MESH_COUNT),
            head: 0,
        }
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

    pub fn inner_metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}

impl Deref for Meshadata {
    type Target = [Metadata];

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, PartialOrd)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

pub const VERTEX_STORAGE_ALLOCATION: usize = 512;
pub const MESH_COUNT: usize = 128;

layout_buffer! {
    const MeshStorage: 2, {
        enum vertex_storage: VERTEX_STORAGE_ALLOCATION => {
            type Vertex;
            bind 0;
            shader 10;
        };

        enum metadata: MESH_COUNT => {
            type Metadata;
            bind 1;
            shader 11;
        };
    }
}

#[derive(Debug)]
pub struct MeshStaging {
    metadata: Meshadata,
    vertex_storage: Vec<Vertex>,
}

impl MeshStaging {
    pub fn new() -> Self {
        Self {
            metadata: Meshadata::new(),
            vertex_storage: Vec::with_capacity(VERTEX_STORAGE_ALLOCATION),
        }
    }

    pub fn stage(&mut self, vertices: &[Vertex]) -> Id {
        self.vertex_storage.extend_from_slice(vertices);
        self.metadata.add(vertices.len() as u32)
    }

    pub fn metadata(&self) -> &Meshadata {
        &self.metadata
    }

    pub fn vertex_storage(&self) -> &[Vertex] {
        &self.vertex_storage
    }

    pub fn close(self) -> Meshadata {
        self.metadata
    }
}

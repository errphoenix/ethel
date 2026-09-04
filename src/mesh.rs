/// The ID that represents a Mesh present on GPU memory, from the CPU.
///
/// An ID of `0` represents a `null` mesh: this is currently an empty mesh, but
/// it could be changed to a "debug" mesh in the future.
///
/// It is used to link objects or "renderables" to a mesh that is present on
/// the GPU through its [`Metadata`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Id(pub(crate) u32);
impl Id {
    pub const unsafe fn from_value(index: u32) -> Self {
        Self(index)
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// The position and length of a Mesh on GPU memory, in terms of triangles.
///
/// This is usually accessed through a [`Mesh ID`](Id).
///
/// It indicates the starting index in the triangle buffer and the total
/// triangle count of the mesh.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Metadata {
    pub(crate) tri_offset: u32,
    pub(crate) tri_count: u32,
}
impl Metadata {
    pub const fn from_values(tri_offset: u32, tri_count: u32) -> Self {
        Self {
            tri_offset,
            tri_count,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Meshadata {
    metadata: Vec<Metadata>,

    /// triangle offset
    head: u32,
}
impl Meshadata {
    pub fn new() -> Self {
        let metadata = vec![Metadata::default()];
        Self { metadata, head: 0 }
    }

    pub fn clear(&mut self) {
        self.metadata.clear();
        self.metadata.push(Metadata::default());
        self.head = 0;
    }

    pub fn add(&mut self, tris: u32) -> Id {
        let id = self.metadata.len() as u32;
        self.metadata.push(Metadata {
            tri_offset: self.head,
            tri_count: tris,
        });
        self.head += tris;
        Id(id)
    }

    pub fn get(&self, id: Id) -> &Metadata {
        &self.metadata[id.0 as usize]
    }

    /// The current head (offset) of the triangle buffer.
    pub fn head(&self) -> u32 {
        self.head
    }

    pub fn inner_metadata(&self) -> &[Metadata] {
        &self.metadata
    }
}
impl std::ops::Deref for Meshadata {
    type Target = [Metadata];

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, PartialOrd)]
pub struct Vertex {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub norm_x: f32,
    pub norm_y: f32,
    pub norm_z: f32,
    pub uv_x: f32,
    pub uv_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, PartialOrd)]
pub struct Triangle {
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
}

crate::shader_glsl_struct! {
    struct MeshMetadata {
        offset: u32 => uint,
        length: u32 => uint
    }
}
crate::shader_glsl_struct! {
    struct MeshVertex {
        pos_x: f32 => float,
        pos_y: f32 => float,
        pos_z: f32 => float,
        norm_x: f32 => float,
        norm_y: f32 => float,
        norm_z: f32 => float,
        uv_x: f32 => float,
        uv_y: f32 => float
    }
}
crate::shader_glsl_struct! {
    struct MeshTriangle {
        v0: u32 => uint,
        v1: u32 => uint,
        v2: u32 => uint
    }
}

macro_rules! ssbo_binding {
    (eth_Mesh_StaticData) => {
        10
    };
    (eth_Mesh_Triangles) => {
        11
    };
}

pub const ETH_MESH_SSBO_BIND_STATICDATA: usize = ssbo_binding!(eth_Mesh_StaticData);
pub const ETH_MESH_SSBO_BIND_TRIANGLES: usize = ssbo_binding!(eth_Mesh_Triangles);

/// Helper macro to initialize GPU SSBO's for mesh data.
///
/// This macro requires only three integer values:
/// * `count` for the total mesh count available for mesh metadata
/// * `vertices` for the total *global* size of the vertex buffer
/// * `tris` for the total *global` size of the triangle/index buffer
///
/// Note how the vertex and tris counts are *global* for all meshes,
/// not per mesh.
///
/// # Examples
/// ```rust,ignore
/// layout_mesh_buffer!(count: 32; vertices: 10_000; tris: 5_000);
/// ```
///
/// The above example will allocate two GPU buffers: the first for mesh
/// metadata for 32 unique meshes; the second for vertex data for a total
/// of 10,000 vertices (and normals); and finally a cap of 5000 triangles,
/// each triangle formed by 3 `u32` indices, as standard.
#[macro_export]
macro_rules! layout_mesh_buffer {
    (count: $mc:expr; vertices: $vc:expr; tris: $tc:expr) => {
        layout_mesh_buffer!(MeshStorage; count: $mc; vertices: $vc; tris: $tc);
    };
    ($name:ident; count: $mc:expr; vertices: $vc:expr; tris: $tc:expr) => {
        paste::paste! {
        layout_buffer! {
            const [< $name Static >]: 2, {
                enum vertex_storage: $vc => {
                    type $crate::mesh::Vertex;
                    bind 0;
                    shader 10;
                };
                enum metadata: $mc => {
                    type $crate::mesh::Metadata;
                    bind 1;
                    shader 10;
                };
            }
        }
        layout_buffer! {
            const [< $name Tris >]: 1, {
                enum tris_storage: $tc => {
                    type $crate::mesh::Triangle;
                    bind 0;
                    shader 11;
                };
            }
        }
        }

        macro_rules! ssbo_binding {
            (eth_Mesh_StaticData) => {
                10
            };
            (eth_Mesh_Triangles) => {
                11
            };
        }

        /// A single GLSL ssbo block with 2 fixed-length arrays for static
        /// geometry data.
        ///
        /// The length matches the capacity provided in the
        /// [`ethel::layout_mesh_buffer`] macro, and will correspond to the
        /// correct offset and lenghts in the [`ethel::render::buffer::Layout`]
        /// generated by the macro.
        ///
        /// The arrays are `eth_vertex_buffer`, over [`MeshVertex`] and
        /// `eth_meshmeta` over [`MeshMetadata`].
        ///
        /// The ssbo is configured on binding index 10.
        pub const ETH_MESH_SSBO_STATIC: $crate::shader::glsl::GlslStorage = $crate::shader_glsl_ssbo! {
            buf eth_Mesh_StaticData => {
                MeshVertex   : eth_vertex_buffer[$vc];
                MeshMetadata : eth_meshmeta[$mc];
            }
        };

        /// A dedicated triangle buffer GLSL ssbo block formed by a single
        /// runtime array.
        ///
        /// The length matches the capacity provided in the
        /// [`ethel::layout_mesh_buffer`] macro, and will correspond to the
        /// correct offset and lenghts in the [`ethel::render::buffer::Layout`]
        /// generated by the macro.
        ///
        /// The array is `eth_tris_buffer`, and its ssbo is configured on
        /// binding index 11.
        pub const ETH_MESH_SSBO_TRIS: $crate::shader::glsl::GlslStorage = $crate::shader_glsl_ssbo! {
            buf eth_Mesh_Triangles => {
                [dyn_array MeshTriangle : eth_tris_buffer]
            }
        };
    };
}

#[derive(Debug, Default)]
pub struct MeshStaging {
    metadata: Meshadata,
    vertex_storage: Vec<Vertex>,
    triangle_storage: Vec<Triangle>,
}
impl MeshStaging {
    pub fn new() -> Self {
        Self::default()
    }

    /// The indices of `triangles` must be local to `vertices`.
    pub fn stage(&mut self, vertices: &[Vertex], triangles: &[Triangle]) -> Id {
        let offset = self.vertex_storage.len() as u32;

        self.vertex_storage.extend_from_slice(vertices);
        self.triangle_storage
            .extend(triangles.iter().map(|tri| Triangle {
                v0: tri.v0 + offset,
                v1: tri.v1 + offset,
                v2: tri.v2 + offset,
            }));

        self.metadata.add(triangles.len() as u32)
    }

    pub fn metadata(&self) -> &Meshadata {
        &self.metadata
    }

    pub fn vertex_storage(&self) -> &[Vertex] {
        &self.vertex_storage
    }

    pub fn triangle_storage(&self) -> &[Triangle] {
        &self.triangle_storage
    }

    pub fn close(self) -> Meshadata {
        self.metadata
    }
}

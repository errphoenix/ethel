pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

pub use state::GpuEntityData;

const ENTITY_ALLOCATION: usize = 512;

layout_buffer! {
    const EntityData = { render::RENDER_STORAGE_PARTS }, {
        // add command buffer
        entity_map => 1, type GpuEntityData = ENTITY_ALLOCATION, shader 1;
        mesh_data => 2, type mesh::Id = ENTITY_ALLOCATION, shader 2;
        transforms => 3, type [f32; 16] = ENTITY_ALLOCATION, shader 3;
    }
}

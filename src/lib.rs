pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

pub use state::GpuEntityData;

const ENTITY_ALLOCATION: usize = 512;

layout_buffer! {
    const EntityData = { render::RENDER_STORAGE_PARTS }, {
        entity_map => 0, type GpuEntityData = ENTITY_ALLOCATION;
        mesh_data => 1, type mesh::Id = ENTITY_ALLOCATION;
        transforms => 2, type [f32; 16] = ENTITY_ALLOCATION;
    }
}

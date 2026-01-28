pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

pub use state::GpuEntityData;

pub const RENDER_STORAGE_PARTS: usize = 4;
pub const ENTITY_ALLOCATION: usize = 512;
pub const COMMAND_QUEUE_ALLOC: usize = 8;

pub type DrawCommand = render::command::DrawArraysIndirectCommand;

layout_buffer! {
    const EntityData = RENDER_STORAGE_PARTS, {
        // add command buffer
        commands => 0, type DrawCommand = COMMAND_QUEUE_ALLOC;
        entity_map => 1, type GpuEntityData = ENTITY_ALLOCATION, shader 1;
        mesh_data => 2, type mesh::Id = ENTITY_ALLOCATION, shader 2;
        transforms => 3, type [f32; 16] = ENTITY_ALLOCATION, shader 3;
    }
}

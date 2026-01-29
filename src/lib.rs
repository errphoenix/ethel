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
    const EntityData: RENDER_STORAGE_PARTS, {
        enum entity_map: ENTITY_ALLOCATION => {
            type DrawCommand;
            bind 0;
            init with {
                DrawCommand::default()
            };
            shader 1;
        };

        enum mesh_data: ENTITY_ALLOCATION => {
            type mesh::Id;
            bind 1;
            shader 2;
        };

        enum positions: ENTITY_ALLOCATION => {
            type [f32; 3];
            bind 2;
            shader 3;
        };
        enum rotations: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 3;
            shader 4;
        };
    }
}

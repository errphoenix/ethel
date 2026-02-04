pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

use janus::input::InputState;
pub use state::GpuEntityMapping;

use crate::render::buffer::{PartitionedTriBuffer, TriBuffer};

pub const RENDER_STORAGE_PARTS: usize = 4;
pub const ENTITY_ALLOCATION: usize = 512;
pub const COMMAND_QUEUE_ALLOC: usize = 64;

pub type InputSystem = InputState<{ janus::input::SLOT_COUNT }, { janus::input::SECTION_COUNT }>;

pub type DrawCommand = render::command::DrawArraysIndirectCommand;

layout_buffer! {
    const EntityData: RENDER_STORAGE_PARTS, {
        enum IMapPositions: ENTITY_ALLOCATION => {
            type u32;
            bind 0;
            shader 2;
        };
        enum IMapRotations: ENTITY_ALLOCATION => {
            type u32;
            bind 1;
            shader 3;
        };
        enum PodPositions: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 2;
            shader 4;
        };
        enum PodRotations: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 3;
            shader 5;
        };
    }
}

#[derive(Debug, Default)]
pub struct FrameStorageBuffers {
    pub command: TriBuffer<DrawCommand>,
    pub scene: PartitionedTriBuffer<RENDER_STORAGE_PARTS>,
}

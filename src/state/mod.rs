use std::time::{Duration, Instant};

use crate::{
    FrameStorageBuffers, LayoutEntityData, RENDER_STORAGE_PARTS, mesh,
    render::{
        buffer::partitioned::PartitionedTriBuffer,
        command::{DrawArraysIndirectCommand, GpuCommandQueue},
    },
    state::{
        column::Column,
        cross::{Cross, Producer},
    },
};

pub mod column;
pub mod cross;

#[derive(Debug)]
struct Entity {
    mesh: u32,
    position: u32,
    rotation: u32,
}

#[derive(Debug, Default)]
pub struct State {
    mesh_ids: Column<mesh::Id>,

    positions: Column<glam::Vec4>,
    rotations: Column<glam::Quat>,

    entities: Vec<Entity>,

    boundary: Cross<Producer, FrameStorageBuffers>,
    command_queue: GpuCommandQueue<crate::DrawCommand>,
}

impl State {
    pub fn boundary(&self) -> &Cross<Producer, FrameStorageBuffers> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Producer, FrameStorageBuffers> {
        &mut self.boundary
    }

    pub fn upload(&mut self) {
        self.boundary.cross(|section, storage| {
            let scene = &storage.scene;
            let index = section.as_index();
            let rotations = self.rotations.iter();

            // unsafe {
            //     scene.blit_part(
            //         section.as_index(),
            //         LayoutEntityData::Rotations as usize,
            //         &rotations,
            //         0,
            //     );
            // }
            //todo
        });
    }

    pub fn command_queue(&self) -> &GpuCommandQueue<crate::DrawCommand> {
        &self.command_queue
    }

    pub fn command_queue_mut(&mut self) -> &mut GpuCommandQueue<crate::DrawCommand> {
        &mut self.command_queue
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuEntityMapping {
    mesh_id_index: u32,
    position_index: u32,
    rotation_index: u32,
    _pad: u32,
}

impl janus::context::Update for State {
    fn update(&mut self, delta: janus::context::DeltaTime) {
        let t0 = Instant::now();
        self.rotations.iter_mut().for_each(|rot| {
            *rot = rot.mul_quat(glam::Quat::from_axis_angle(
                glam::Vec3::Y,
                delta.as_f32() * 10f32,
            ));
        });

        self.upload();

        let t1 = Instant::now();
        println!("logic thread time: {}", (t1 - t0).as_nanos());
    }

    fn step_duration(&self) -> std::time::Duration {
        //todo
        Duration::from_millis(6)
    }

    fn set_step_duration(&mut self, _step: std::time::Duration) {
        //todo
    }
}

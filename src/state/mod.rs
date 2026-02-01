use std::time::{Duration, Instant};

use crate::{
    FrameStorageBuffers, LayoutEntityData, mesh,
    render::command::{DrawArraysIndirectCommand, GpuCommandQueue},
    state::{
        column::{Column, IterColumn, ParallelIndexArrayColumn},
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
    mesh_ids: Vec<mesh::Id>,

    positions: ParallelIndexArrayColumn<glam::Vec4>,
    rotations: ParallelIndexArrayColumn<glam::Quat>,

    entities: Vec<Entity>,

    boundary: Cross<Producer, FrameStorageBuffers>,
    command_queue: GpuCommandQueue<crate::DrawCommand>,
}

impl State {
    // todo: change to return an entity handle to wrap around raw index
    // and maybe generation
    pub fn create_entity(
        &mut self,
        // should likely pass a "mesh name" or handle instead instead of raw index
        mesh_handle: usize,
        position: impl Into<glam::Vec4>,
        rotation: impl Into<glam::Quat>,
    ) -> usize {
        let position_id = self.positions.put(position.into());
        let rotation_id = self.rotations.put(rotation.into());
        let entity_id = self.entities.len();
        self.entities.push(Entity {
            mesh: mesh_handle as u32,
            position: position_id,
            rotation: rotation_id,
        });
        entity_id
    }

    pub fn boundary(&self) -> &Cross<Producer, FrameStorageBuffers> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Producer, FrameStorageBuffers> {
        &mut self.boundary
    }

    pub fn upload(&mut self) {
        self.command_queue.push(DrawArraysIndirectCommand {
            count: 3,
            instance_count: 1,
            first_vertex: 0,
            base_instance: 0,
        });

        self.boundary.cross(|section, storage| {
            let scene = &storage.scene;
            let index = section.as_index();

            let i_positions = self.positions.handles();
            let i_rotations = self.rotations.handles();
            let positions = self.positions.contiguous();
            let rotations = self.rotations.contiguous();

            unsafe {
                scene.blit_part(
                    index,
                    LayoutEntityData::ImapPositions as usize,
                    i_positions,
                    0,
                );
                scene.blit_part(
                    index,
                    LayoutEntityData::ImapRotations as usize,
                    i_rotations,
                    0,
                );
                scene.blit_part(index, LayoutEntityData::PodPositions as usize, positions, 0);
                scene.blit_part(index, LayoutEntityData::PodRotations as usize, rotations, 0);
            }
        });
    }

    pub fn command_queue(&self) -> &GpuCommandQueue<crate::DrawCommand> {
        &self.command_queue
    }

    pub fn command_queue_mut(&mut self) -> &mut GpuCommandQueue<crate::DrawCommand> {
        &mut self.command_queue
    }

    pub fn global_mesh_storage(&self) -> &[mesh::Id] {
        &self.mesh_ids
    }

    pub fn global_mesh_storage_mut(&mut self) -> &mut Vec<mesh::Id> {
        &mut self.mesh_ids
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

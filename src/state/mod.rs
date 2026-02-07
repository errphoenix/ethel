use std::time::{Duration, Instant};

use tracing::{Level, event};

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
pub mod table;

/// An entity is simply a series of handles in one or more columns or tables.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Entity {
    // the direct index in the mesh_ids vector
    mesh: u32,

    // the indirect index in the positions column
    position: u32,
    // the indirect index in the rotations column
    rotation: u32,
    _pad: u32,
}

#[derive(Debug, Default)]
pub struct State {
    input: crate::InputSystem,

    // immutable mesh IDs of GPU-side mesh data, loaded during init
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
            _pad: 0,
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
        self.entities.iter().for_each(|_| {
            self.command_queue.push(DrawArraysIndirectCommand {
                count: 3,
                instance_count: 1,
                first_vertex: 0,
                base_instance: 0,
            });
        });

        self.boundary.cross(|section, storage| {
            let index = section.as_index();

            {
                let scene = &storage.scene;

                let entity_map = &self.entities;
                let mesh_map = &self.mesh_ids;
                let i_positions = self.positions.handles();
                let i_rotations = self.rotations.handles();
                let positions = self.positions.contiguous();
                let rotations = self.rotations.contiguous();

                unsafe {
                    scene.blit_part(
                        index,
                        LayoutEntityData::EntityIndexMap as usize,
                        entity_map,
                        0,
                    );
                    scene.blit_part(index, LayoutEntityData::MeshData as usize, mesh_map, 0);

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
            }

            {
                let command = &storage.command;
                let mut data = command.view_section_mut(index);
                if let Err(overflow) = self.command_queue().upload(&mut data) {
                    event!(
                        name: "render.command.upload.overflow",
                        Level::WARN,
                        "render command queue overflow during upload: {overflow} commands could not be uploaded and will be discarded"
                    );
                }
            }
        });

        self.command_queue_mut().clear();
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

    pub fn input(&self) -> &crate::InputSystem {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut crate::InputSystem {
        &mut self.input
    }
}

impl janus::context::Update for State {
    fn update(&mut self, delta: janus::context::DeltaTime) {
        let t0 = Instant::now();

        self.input.poll_key_events();
        if self
            .input()
            .keys()
            .mouse_down(janus::input::MouseButton::Left)
        {
            println!(
                "{}",
                self.input()
                    .keys()
                    .mouse_frames_held(janus::input::MouseButton::Left)
            );
        }

        self.rotations.iter_mut().for_each(|rot| {
            *rot = rot.mul_quat(glam::Quat::from_axis_angle(
                glam::Vec3::Y,
                delta.as_f32() * 10f32,
            ));
        });

        self.upload();

        let t1 = Instant::now();
        println!(
            "logic thread time: {} nanos / FPS: {}",
            (t1 - t0).as_nanos(),
            (1_000_000_000 / (t1 - t0).as_nanos())
        );
    }

    fn step_duration(&self) -> std::time::Duration {
        //todo
        Duration::from_millis(6)
    }

    fn set_step_duration(&mut self, _step: std::time::Duration) {
        //todo
    }

    fn new_frame(&mut self) {
        self.input.sync();
    }
}

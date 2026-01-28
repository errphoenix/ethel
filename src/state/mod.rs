use std::time::Duration;

use crate::{
    mesh,
    render::{self, data::RenderStorage},
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
    meshes: Column<mesh::Id>,
    positions: Column<glam::Vec3>,

    entities: Vec<Entity>,

    boundary: Cross<Producer, RenderStorage<{ render::RENDER_STORAGE_PARTS }>>,
}

impl State {
    pub fn boundary(&self) -> &Cross<Producer, RenderStorage<{ render::RENDER_STORAGE_PARTS }>> {
        &self.boundary
    }

    pub fn boundary_mut(
        &mut self,
    ) -> &mut Cross<Producer, RenderStorage<{ render::RENDER_STORAGE_PARTS }>> {
        &mut self.boundary
    }

    pub fn upload(&mut self) {
        self.boundary.cross(|section, storage| {
            let mut matrices = unsafe { storage.view_part_mut::<[f32; 16]>(section as usize, 0) };
            self.pack_matrices(&mut matrices);
        });
    }

    fn pack_matrices(&self, out: &mut [[f32; 16]]) {
        self.positions.direct().iter().for_each(|item| {
            let idx = item.owner() as usize;
            let pos = item.inner_value();
            out[idx] = glam::Mat4::from_translation(*pos).to_cols_array();
        });
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuEntityData {
    mesh_id_index: u32,
    transform_index: u32,
}

impl janus::context::Update for State {
    fn update(&mut self, delta: janus::context::DeltaTime) {
        self.positions
            .iter_mut()
            .for_each(|pos| pos.x += delta.as_f32() * 0.1);
        //todo

        self.upload();
    }

    fn step_duration(&self) -> std::time::Duration {
        //todo
        Duration::from_millis(6)
    }

    fn set_step_duration(&mut self, _step: std::time::Duration) {
        //todo
    }
}

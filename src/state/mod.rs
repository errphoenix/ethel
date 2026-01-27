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
struct Renderable {
    mesh: u32,
    position: u32,
    rotation: u32,
}

#[derive(Debug, Default)]
pub struct State {
    meshes: Column<mesh::Id>,
    positions: Column<glam::Vec3>,

    renderables: Vec<Renderable>,

    boundary: Cross<Producer, RenderStorage<{ render::RENDER_STORAGE_PARTS }>>,
    transforms: Box<glam::Mat4>,
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

    pub fn upload(&self) {
        self.boundary.cross(|section, storage| {});
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuEntityData {
    mesh: u32,
    transform: u32,
}

impl janus::context::Update for State {
    fn update(&mut self, _delta: janus::context::DeltaTime) {
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

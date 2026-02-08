use janus::sync::Mirror;

use crate::{
    StateHandler,
    render::command::GpuCommandQueue,
    state::{
        camera::ViewPoint,
        cross::{Cross, Producer},
    },
};

pub mod camera;
pub mod cross;
pub mod data;

#[derive(Debug, Default)]
pub struct State<D: Sized, T: StateHandler<D>> {
    input: crate::InputSystem,

    view: Mirror<ViewPoint>,
    handler: T,

    boundary: Cross<Producer, D>,
    cmd_queue: GpuCommandQueue<crate::DrawCommand>,
}

pub(crate) const DEFAULT_STEP: std::time::Duration = std::time::Duration::from_millis(8);

impl<D: Sized, T: StateHandler<D>> State<D, T> {
    pub fn boundary(&self) -> &Cross<Producer, D> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Producer, D> {
        &mut self.boundary
    }

    pub fn upload(&mut self) {
        self.handler.upload_gpu(&self.boundary, &mut self.cmd_queue);
    }

    pub fn command_queue(&self) -> &GpuCommandQueue<crate::DrawCommand> {
        &self.cmd_queue
    }

    pub fn command_queue_mut(&mut self) -> &mut GpuCommandQueue<crate::DrawCommand> {
        &mut self.cmd_queue
    }

    pub fn input(&self) -> &crate::InputSystem {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut crate::InputSystem {
        &mut self.input
    }

    pub fn viewpoint_mirror(&self) -> &Mirror<ViewPoint> {
        &self.view
    }

    pub fn viewpoint_mirror_mut(&mut self) -> &mut Mirror<ViewPoint> {
        &mut self.view
    }
}

impl<D: Sized, T: StateHandler<D>> janus::context::Update for State<D, T> {
    #[inline]
    fn update(&mut self, delta: janus::context::DeltaTime) {
        self.input.poll_key_events();
        self.handler.step(&self.input, &mut self.view, delta);
        self.upload();
    }

    #[inline]
    fn step_duration(&self) -> std::time::Duration {
        self.handler.step_duration()
    }

    #[inline]
    fn new_frame(&mut self) {
        self.input.sync();
        self.handler.on_new_frame();
    }
}

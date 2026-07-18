use std::sync::Arc;

use janus::sync;

use crate::{
    StateHandler,
    render::{
        ScreenSpace,
        command::{DrawGroups, GpuCommandQueue},
    },
    state::{
        camera::ViewPoint,
        cross::{Cross, Producer},
    },
};

pub mod camera;
pub mod cross;
pub mod data;
pub mod time;

#[derive(Debug)]
pub struct State<D: Sized, T: StateHandler<D, RG>, RG: DrawGroups> {
    input: crate::InputSystem,

    screen: sync::Mirror<ScreenSpace>,
    view: Arc<sync::TriCell<ViewPoint>>,
    handler: T,

    boundary: Cross<Producer, D>,
    cmd_queue: GpuCommandQueue<crate::DrawCommand, RG>,
}

impl<D, T, RG> Default for State<D, T, RG>
where
    D: Sized + Default,
    T: StateHandler<D, RG> + Default,
    RG: DrawGroups,
{
    fn default() -> Self {
        Self {
            input: Default::default(),
            screen: Default::default(),
            view: Default::default(),
            handler: Default::default(),
            boundary: Default::default(),
            cmd_queue: GpuCommandQueue::new(),
        }
    }
}

pub(crate) const DEFAULT_STEP: std::time::Duration = std::time::Duration::from_millis(8);

impl<D, T, RG> State<D, T, RG>
where
    D: Sized,
    T: StateHandler<D, RG>,
    RG: DrawGroups,
{
    pub fn handler_init_callback<F: FnOnce(&mut T)>(&mut self, callback: F) {
        callback(&mut self.handler)
    }

    pub fn boundary(&self) -> &Cross<Producer, D> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Producer, D> {
        &mut self.boundary
    }

    pub fn upload(&mut self) {
        self.handler.upload_gpu(&self.boundary, &mut self.cmd_queue);
    }

    pub fn command_queue(&self) -> &GpuCommandQueue<crate::DrawCommand, RG> {
        &self.cmd_queue
    }

    pub fn command_queue_mut(&mut self) -> &mut GpuCommandQueue<crate::DrawCommand, RG> {
        &mut self.cmd_queue
    }

    pub fn input(&self) -> &crate::InputSystem {
        &self.input
    }

    pub fn input_mut(&mut self) -> &mut crate::InputSystem {
        &mut self.input
    }

    pub fn viewpoint(&self) -> &ViewPoint {
        &self.view
    }

    pub fn viewpoint_shared(&self) -> &Arc<sync::TriCell<ViewPoint>> {
        &self.view
    }

    pub fn screen_space(&self) -> &ScreenSpace {
        &self.screen
    }

    pub fn screen_space_mirror(&self) -> &sync::Mirror<ScreenSpace> {
        &self.screen
    }

    pub fn screen_space_mirror_mut(&mut self) -> &mut sync::Mirror<ScreenSpace> {
        &mut self.screen
    }
}

impl<D, T, RG> janus::context::Update for State<D, T, RG>
where
    D: Sized,
    T: StateHandler<D, RG>,
    RG: DrawGroups,
{
    #[inline]
    fn update(&mut self, delta: janus::context::DeltaTime) {
        self.handler
            .fixed_step(&mut self.input, &mut self.screen, &self.view, delta);
    }

    #[inline]
    fn step_duration(&self) -> std::time::Duration {
        self.handler.step_duration()
    }

    #[inline]
    fn new_frame(&mut self, delta: janus::context::DeltaTime) {
        self.input.sync();
        self.input.poll_key_events();

        while let Some(event) = self.input.pop_key_event() {
            self.handler.on_key_event(event);
        }

        self.handler
            .on_new_frame(&mut self.input, &mut self.screen, &self.view, delta);
    }

    fn finish_frame(&mut self) {
        self.upload();
    }
}

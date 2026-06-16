pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

#[cfg(feature = "profile")]
pub mod profile;

#[cfg(feature = "assets")]
pub mod assets;

use janus::{
    input::{InputState, KeyEvent},
    sync::{Mirror, TriCell},
};

use crate::{
    mesh::MeshStaging,
    render::{
        Renderer, Resolution, ScreenSpace,
        buffer::{self, Layout, StorageSection},
        command::{DrawGroups, GpuCommandQueue},
    },
    state::{
        State,
        camera::ViewPoint,
        cross::{self, Cross, Producer},
    },
};

pub type InputSystem = InputState<{ janus::input::SLOT_COUNT }, { janus::input::SECTION_COUNT }>;

pub type DrawCommand = render::command::DrawArraysIndirectCommand;

/// Manages the simulation side state of the program, which contains multiple
/// responsabilities.
///
/// It takes care, first and foremost, of the simulation itself through an
/// obligatory implementation of the [`Self::step`] function. The step
/// function may be called multiple times in one frame, depending on the
/// simulation speed and the capabilities of the system to "keep up".
///
/// Another essential function is [`Self::upload_gpu`], which is the 'write'
/// phase of the GPU synchronization routine. This must write to the provided
/// `frame_boundary` any data that must be present on the gpu.
///
/// An optional but noteworthy function is [`Self::step_duration`], this is a
/// getter function for a [`std::time::Duration`] type. This is already
/// implemented by default, returning [`state::DEFAULT_STEP`] (8ms).
/// This directly impacts the pacing of the [`Self::step`] function.
///
/// There is an optional [`Self::on_new_frame`] function, which is called
/// exactly once for every new frame, differently from [`Self::step`] which
/// is likely to be called multiple times. The default implementation is blank.
///
/// Finally, theres an optional [`Self::on_key_event`] function, which is fed
/// every frame with new [`janus::input::KeyEvent`] with keyboard/mouse button
/// down/release events. This is used to register the pressing of arbitrary
/// keys (for example a text field) which cannot be done with the classic
/// 'is_key_down' approach. The default implementation is blank.
pub trait StateHandler<FrameData: Sized, RG: DrawGroups> {
    /// The 'write' phase of the GPU synchronization routine.
    ///
    /// Write must occur to the given `frame_boundary` and `command_queue`.
    ///
    /// This is called in cohesion with the [`Self::step`] function immediately
    /// after it returns.
    fn upload_gpu(
        &mut self,
        frame_boundary: &Cross<Producer, FrameData>,
        command_queue: &mut GpuCommandQueue<crate::DrawCommand, RG>,
    );

    /// The simulation advance/step routine.
    fn step(
        &mut self,
        input: &mut crate::InputSystem,
        screen: &mut Mirror<ScreenSpace>,
        view_point: &TriCell<ViewPoint>,
        delta: janus::context::DeltaTime,
    );

    fn step_duration(&self) -> std::time::Duration {
        state::DEFAULT_STEP
    }

    /// Sequential keyboard/mouse button processing.
    ///
    /// Useful for arbitrary key events, for ex. for text fields, which would
    /// not be appropriate to implement with the classic 'is_key_down'
    /// approach.
    ///
    /// The function is called continuously for every new `event` occurred
    /// between the last frame and the current frame, in the same order as
    /// they were registered.
    ///
    /// This function is called before the [`Self::step`] function, which is
    /// then called only after all events have been exhausted.
    fn on_key_event(&mut self, _event: KeyEvent) {}

    fn on_new_frame(&mut self) {}
}

pub trait RenderHandler<FrameData: Sized> {
    fn init_resources(&mut self, resolution: Resolution);

    fn pre_frame(
        &mut self,
        screen: &mut Mirror<ScreenSpace>,
        view: &TriCell<ViewPoint>,
        delta: janus::context::DeltaTime,
    );

    fn render_frame(&self, frame_data: &FrameData, section: StorageSection);
}

pub struct StartupHandler<FrameData: Sized> {
    input_system: crate::InputSystem,

    frame_data_init: fn() -> FrameData,
    gl_state_init: fn(),

    mesh_data: MeshStaging,
    mesh_buf_layout: Layout<2>,
}

impl<FrameData: Sized> StartupHandler<FrameData> {
    pub fn new(input_system: crate::InputSystem, init_fn: fn() -> FrameData) -> Self {
        Self {
            input_system,
            frame_data_init: init_fn,
            gl_state_init: || (),
            mesh_data: MeshStaging::new(),
            mesh_buf_layout: Layout::new(),
        }
    }

    pub fn with_mesh_layout(&mut self, mesh_buf_layout: Layout<2>) {
        self.mesh_buf_layout = mesh_buf_layout;
    }

    pub fn with_mesh_data(&mut self, mesh_data: MeshStaging) {
        self.mesh_data = mesh_data;
    }

    pub fn with_gl_state(&mut self, init_fn: fn()) {
        self.gl_state_init = init_fn;
    }
}

impl<Fd, Sh, Rh, RG> janus::context::Setup<State<Fd, Sh, RG>, Renderer<Fd, Rh>>
    for StartupHandler<Fd>
where
    Fd: Sized + Default,
    Sh: StateHandler<Fd, RG> + Default,
    Rh: RenderHandler<Fd> + Default,
    RG: DrawGroups,
{
    fn init(
        self,
        state: &mut State<Fd, Sh, RG>,
        renderer: &mut Renderer<Fd, Rh>,
    ) -> Result<(), &'static str>
    where
        Self: Sized,
    {
        *state.input_mut() = self.input_system;

        {
            let mut mesh_buf = buffer::immutable::uninit(self.mesh_buf_layout);

            let vertices = self.mesh_data.vertex_storage();
            let vbs = mesh::BUFFER_VERTEX_STORAGE_INDEX;
            mesh_buf.fill_partition(vbs, vertices);

            let metadata = self.mesh_data.close();
            let mds = mesh::BUFFER_MESH_META_INDEX;
            mesh_buf.fill_partition(mds, &metadata);

            renderer.mesh_buffer = mesh_buf.finish();
        }

        let m_vp = state.viewpoint_shared().clone();
        renderer.viewpoint = m_vp;

        let frame_data = (self.frame_data_init)();
        let (producer, consumer) = cross::create(frame_data);

        renderer.boundary = consumer;
        *state.boundary_mut() = producer;
        *state.command_queue_mut() = GpuCommandQueue::new();

        (self.gl_state_init)();

        let screen = renderer.screen_space_mirror().clone();
        renderer.handler.init_resources(screen.resolution());
        *state.screen_space_mirror_mut() = screen;
        Ok(())
    }
}

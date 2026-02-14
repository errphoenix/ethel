pub mod mesh;
pub mod render;
pub mod shader;
pub mod state;

use janus::{input::InputState, sync::Mirror};

use crate::{
    mesh::MeshStaging,
    render::{
        Renderer, Resolution,
        buffer::{self, Layout, StorageSection},
        command::GpuCommandQueue,
    },
    state::{
        State,
        camera::ViewPoint,
        cross::{self, Cross, Producer},
    },
};

pub type InputSystem = InputState<{ janus::input::SLOT_COUNT }, { janus::input::SECTION_COUNT }>;

pub type DrawCommand = render::command::DrawArraysIndirectCommand;

pub trait StateHandler<FrameData: Sized> {
    const COMMAND_QUEUE_LENGTH: usize;

    fn upload_gpu(
        &mut self,
        frame_boundary: &Cross<Producer, FrameData>,
        command_queue: &mut GpuCommandQueue<crate::DrawCommand>,
    );

    fn step(
        &mut self,
        input: &mut crate::InputSystem,
        view_point: &mut Mirror<ViewPoint>,
        delta: janus::context::DeltaTime,
    );

    fn step_duration(&self) -> std::time::Duration {
        state::DEFAULT_STEP
    }

    fn on_new_frame(&mut self) {}
}

pub trait RenderHandler<FrameData: Sized> {
    fn init_resources(&mut self, resolution: Resolution);

    fn pre_frame(
        &mut self,
        resolution: Resolution,
        view_point: &mut Mirror<ViewPoint>,
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

impl<Fd, Sh, Rh> janus::context::Setup<State<Fd, Sh>, Renderer<Fd, Rh>> for StartupHandler<Fd>
where
    Fd: Sized + Default,
    Sh: StateHandler<Fd> + Default,
    Rh: RenderHandler<Fd> + Default,
{
    fn init(
        self,
        state: &mut State<Fd, Sh>,
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

            *renderer.mesh_buffer_mut() = mesh_buf.finish();
        }

        let m_vp = state.viewpoint_mirror().clone();
        *renderer.viewpoint_mirror_mut() = m_vp;

        let frame_data = (self.frame_data_init)();
        let (producer, consumer) = cross::create(frame_data);
        *state.boundary_mut() = producer;
        *renderer.boundary_mut() = consumer;

        *state.command_queue_mut() = GpuCommandQueue::new(Sh::COMMAND_QUEUE_LENGTH);

        (self.gl_state_init)();

        renderer.handler.init_resources(renderer.resolution());

        Ok(())
    }
}

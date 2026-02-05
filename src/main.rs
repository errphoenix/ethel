use std::io::BufReader;

use ethel::{
    FrameStorageBuffers, LayoutEntityData,
    mesh::{LayoutMeshStorage, MeshStaging, Vertex},
    render::{
        Renderer,
        buffer::{self, InitStrategy, TriBuffer, partitioned::PartitionedTriBuffer},
        command::GpuCommandQueue,
    },
    shader::ShaderHandle,
    state::{State, cross},
};
use janus::window::DisplayParameters;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let display_params = DisplayParameters::fullscreen("ethel");
    let (input_state, input_dispatch) = janus::input::stream();

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| setup(input_state, state, renderer),
        input_dispatch,
        display_params,
    );

    janus::run(ctx);
}

fn setup(
    input_state: ethel::InputSystem,
    state: &mut State,
    renderer: &mut Renderer,
) -> Result<(), &'static str> {
    *state.input_mut() = input_state;

    {
        let triangle = [
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.33, -0.33, 0.33, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, 0.0, 0.0, 1.0],
                normal: [-0.33, -0.33, 0.33, 1.0],
            },
        ];

        let mut stage = MeshStaging::new();
        let triangle = stage.stage(&triangle);

        let mut mesh_buffer = buffer::immutable::uninit(LayoutMeshStorage::create());
        let vbs = LayoutMeshStorage::VertexStorage;
        mesh_buffer.fill_partition(vbs as usize, stage.vertex_storage());

        let metadata = stage.close();
        let mds = LayoutMeshStorage::Metadata;
        mesh_buffer.fill_partition(mds as usize, &metadata);

        *renderer.mesh_buffer_mut() = mesh_buffer.finish();

        //todo: move mesh IDs to a global map and initialise entities

        *state.global_mesh_storage_mut() = vec![triangle];
    }

    {
        // todo: handle mesh id handles properly

        state.create_entity(0, (0.0, 0.0, -5.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (0.0, 0.0, 5.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (0.0, 3.0, -5.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (10.0, 1.0, -4.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (5.0, 5.0, 0.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (-5.0, 2.0, 0.0, 1.0), glam::Quat::IDENTITY);
        state.create_entity(0, (0.0, 2.0, -1.0, 1.0), glam::Quat::IDENTITY);
    }

    {
        let command = TriBuffer::new_zeroed(ethel::COMMAND_QUEUE_ALLOC, InitStrategy::Zero);
        let scene = PartitionedTriBuffer::new(LayoutEntityData::create());

        let frame_storage_buffers = FrameStorageBuffers { command, scene };

        let (producer, consumer) = cross::create(frame_storage_buffers);
        *state.boundary_mut() = producer;
        *renderer.boundary_mut() = consumer;
    }
    {
        let mut vsh = BufReader::new(include_bytes!("shader/base.vsh").as_slice());
        let mut fsh = BufReader::new(include_bytes!("shader/base.fsh").as_slice());
        let shader = ShaderHandle::new(&mut vsh, &mut fsh);
        renderer.set_shader_handle(shader);
    }

    {
        *state.command_queue_mut() = GpuCommandQueue::new(ethel::COMMAND_QUEUE_ALLOC);
    }
    Ok(())
}

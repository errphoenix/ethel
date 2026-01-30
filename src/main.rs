use std::io::BufReader;

use ethel::{
    FrameStorageBuffers, LayoutEntityData,
    render::{
        Renderer,
        buffer::{InitStrategy, TriBuffer, partitioned::PartitionedTriBuffer},
        command::GpuCommandQueue,
    },
    shader::ShaderHandle,
    state::{State, cross},
};
use janus::window::DisplayParameters;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let display_params = DisplayParameters::windowed("ethel", 1920, 1080);
    let ctx = janus::context::Context::new(setup, display_params);
    janus::run(ctx);
}

fn setup(state: &mut State, renderer: &mut Renderer) -> anyhow::Result<()> {
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

    unsafe {
        janus::gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    }
    Ok(())
}

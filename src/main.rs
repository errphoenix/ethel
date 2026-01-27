use std::io::BufReader;

use ethel::{render::Renderer, shader::ShaderHandle, state::State};
use janus::window::DisplayParameters;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let display_params = DisplayParameters::windowed("ethel", 1920, 1080);
    let ctx = janus::context::Context::new(setup, display_params);
    janus::run(ctx);
}

fn setup(_state: &mut State, renderer: &mut Renderer) -> anyhow::Result<()> {
    renderer.set_shader_handle(load_shader());
    Ok(())
}

fn load_shader() -> ShaderHandle {
    let mut vsh = BufReader::new(include_bytes!("shader/base.vsh").as_slice());
    let mut fsh = BufReader::new(include_bytes!("shader/base.fsh").as_slice());
    ShaderHandle::new(&mut vsh, &mut fsh)
}

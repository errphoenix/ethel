pub mod buffer;
pub mod command;
pub mod sync;

use std::time::Instant;

use glam::{Mat4, Vec4Swizzles};

use crate::{
    FrameStorageBuffers,
    mesh::Meshadata,
    render::{buffer::ImmutableBuffer, command::GpuCommandDispatch, sync::SyncBarrier},
    shader::ShaderHandle,
    state::cross::{Consumer, Cross},
};

pub trait GlPropertyEnum {
    fn as_gl_enum(&self) -> u32;
}

const ORTHO_NEAR: f32 = 0.0;
const ORTHO_FAR: f32 = 2.0;
const PERSP_NEAR: f32 = 0.1;

pub(crate) fn projection_orthographic(width: f32, height: f32) -> Mat4 {
    Mat4::orthographic_rh_gl(0.0, width, height, 0.0, ORTHO_NEAR, ORTHO_FAR)
}

pub(crate) fn projection_perspective(width: f32, height: f32, fov_degrees: f32) -> Mat4 {
    Mat4::perspective_infinite_reverse_rh(fov_degrees.to_radians(), width / height, PERSP_NEAR)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Resolution {
    dirty: bool,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewPoint {
    transform: glam::Mat4,
}

impl ViewPoint {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_position(pos: glam::Vec3) -> Self {
        Self {
            transform: glam::Mat4::from_translation(-pos),
        }
    }

    pub fn replace_transform(&mut self, transform: glam::Mat4) -> glam::Mat4 {
        std::mem::replace(&mut self.transform, transform)
    }

    pub fn to_scale_rotation_translation(&self) -> (glam::Vec3, glam::Quat, glam::Vec3) {
        self.transform.to_scale_rotation_translation()
    }

    pub fn translation(&self) -> glam::Vec3 {
        self.transform.w_axis.xyz()
    }

    pub fn translation_mut(&mut self) -> &mut glam::Vec4 {
        &mut self.transform.w_axis
    }

    pub fn transform(&self) -> &glam::Mat4 {
        &self.transform
    }

    pub fn transform_mut(&mut self) -> &mut glam::Mat4 {
        &mut self.transform
    }
}

impl Resolution {
    pub fn is_changed(&self) -> bool {
        self.dirty
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn to_half(&self) -> Resolution {
        Resolution {
            width: self.width / 2f32,
            height: self.height / 2f32,
            dirty: true,
        }
    }

    pub fn to_double(&self) -> Resolution {
        Resolution {
            width: self.width * 2f32,
            height: self.height * 2f32,
            dirty: true,
        }
    }

    pub fn to_quarter(&self) -> Resolution {
        Resolution {
            width: self.width / 4f32,
            height: self.height / 4f32,
            dirty: true,
        }
    }
}

/// Render state for the Janus rendering Context
#[derive(Debug, Default)]
pub struct Renderer {
    // only used for rendering as sometimes opengl may refuse to draw anything
    // without a vao bound during draw calls
    render_vao: u32,

    mesh_buffer: ImmutableBuffer<2>,
    pub(crate) metadata: Meshadata,

    resolution: Resolution,
    pub(crate) view: ViewPoint,

    shader: ShaderHandle,

    sync_barrier: SyncBarrier,
    boundary: Cross<Consumer, FrameStorageBuffers>,
}

impl Renderer {
    pub fn mesh_buffer(&self) -> &ImmutableBuffer<2> {
        &self.mesh_buffer
    }

    pub fn mesh_buffer_mut(&mut self) -> &mut ImmutableBuffer<2> {
        &mut self.mesh_buffer
    }

    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn view(&self) -> &ViewPoint {
        &self.view
    }

    pub fn view_mut(&mut self) -> &mut ViewPoint {
        &mut self.view
    }

    pub fn metadata(&self) -> &Meshadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Meshadata {
        &mut self.metadata
    }

    pub fn shader_handle(&self) -> &ShaderHandle {
        &self.shader
    }

    pub fn set_shader_handle(&mut self, shader: ShaderHandle) {
        self.shader = shader;
    }

    pub fn boundary(&self) -> &Cross<Consumer, FrameStorageBuffers> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Consumer, FrameStorageBuffers> {
        &mut self.boundary
    }

    pub fn viewpoint_mirror(&self) -> &Mirror<ViewPoint> {
        &self.view
    }

    pub fn viewpoint_mirror_mut(&mut self) -> &mut Mirror<ViewPoint> {
        &mut self.view
    }
}

const FOV: f32 = 80.0;

impl janus::context::Draw for Renderer {
    fn draw(&mut self, _delta: janus::context::DeltaTime) {
        let t0 = Instant::now();

        if self.render_vao == 0 {
            unsafe {
                janus::gl::GenVertexArrays(1, &mut self.render_vao);
                janus::gl::BindVertexArray(self.render_vao);
            }
        }
        if self.resolution.is_changed() {
            self.resolution.dirty = false;
            let w = self.resolution.width as i32;
            let h = self.resolution.height as i32;

            unsafe {
                janus::gl::Viewport(0, 0, w, h);
            }
        }

        unsafe {
            janus::gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT);
        }

        let t0 = Instant::now();

        {
            let proj = projection_perspective(self.resolution.width, self.resolution.height, FOV);
            let view_transform = self.view.transform;
            self.shader.bind();
            self.shader.uniform_mat4_glam("u_view", view_transform);
            self.shader.uniform_mat4_glam("u_projection", proj);
        }

        //todo

        self.boundary
            .cross(&mut self.sync_barrier, |section, storage| {
                self.mesh_buffer.bind_shader_storage();

                let scene = &storage.scene;
                scene.bind_shader_storage(section.as_index());

                let cmd = storage.command.view_section(section.as_index());
                GpuCommandDispatch::from_view(cmd).dispatch();
            });

        let t1 = Instant::now();

        println!(
            "render thread time: {} nanos / FPS: {}",
            (t1 - t0).as_nanos(),
            (1_000_000_000 / (t1 - t0).as_nanos())
        );

        #[cfg(debug_assertions)]
        {
            #[allow(unused_assignments)]
            let mut err = 0;
            loop {
                use tracing::Level;

                err = unsafe { janus::gl::GetError() };
                if err == 0 {
                    break;
                }

                tracing::event!(
                    name: "render.debug.gl_err",
                    Level::DEBUG,
                    "gl error: {err}"
                );
            }
        }
    }

    fn set_resolution(&mut self, (w, h): (f32, f32)) {
        self.resolution.dirty = true;
        self.resolution.width = w;
        self.resolution.height = h;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            janus::gl::DeleteVertexArrays(1, &self.render_vao);
        }
    }
}

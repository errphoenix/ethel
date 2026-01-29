pub mod buffer;
pub mod command;
pub mod sync;

use glam::{Mat4, Vec4Swizzles};

use crate::{
    RENDER_STORAGE_PARTS,
    mesh::Meshadata,
    render::{
        buffer::partitioned::PartitionedTriBuffer,
        command::{DrawArraysIndirectCommand, GpuCommandQueue},
        sync::SyncBarrier,
    },
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
    width: f32,
    height: f32,
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
        }
    }

    pub fn to_double(&self) -> Resolution {
        Resolution {
            width: self.width * 2f32,
            height: self.height * 2f32,
        }
    }

    pub fn to_quarter(&self) -> Resolution {
        Resolution {
            width: self.width / 4f32,
            height: self.height / 4f32,
        }
    }
}

/// Render state for the Janus rendering Context
#[derive(Debug, Default)]
pub struct Renderer {
    resolution: Resolution,

    pub(crate) metadata: Meshadata,
    pub(crate) view: ViewPoint,

    shader: ShaderHandle,
    command_queue: GpuCommandQueue<DrawArraysIndirectCommand>,

    sync_barrier: SyncBarrier,
    boundary: Cross<Consumer, PartitionedTriBuffer<RENDER_STORAGE_PARTS>>,
}

impl Renderer {
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn set_resolution(&mut self, resolution: Resolution) {
        self.resolution = resolution;
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

    pub fn boundary(&self) -> &Cross<Consumer, PartitionedTriBuffer<RENDER_STORAGE_PARTS>> {
        &self.boundary
    }

    pub fn boundary_mut(
        &mut self,
    ) -> &mut Cross<Consumer, PartitionedTriBuffer<RENDER_STORAGE_PARTS>> {
        &mut self.boundary
    }

    pub fn command_queue(&self) -> &GpuCommandQueue<DrawArraysIndirectCommand> {
        &self.command_queue
    }

    pub fn command_queue_mut(&mut self) -> &mut GpuCommandQueue<DrawArraysIndirectCommand> {
        &mut self.command_queue
    }
}

const FOV: f32 = 80.0;

impl janus::context::Draw for Renderer {
    fn draw(&mut self, delta: janus::context::DeltaTime) {
        unsafe {
            janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT);
        }

        {
            let proj = projection_perspective(self.resolution.width, self.resolution.height, FOV);
            let view_transform = self.view.transform;
            self.shader.uniform_mat4_glam("u_view", view_transform);
            self.shader.uniform_mat4_glam("u_projection", proj);
        }

        *self.view.translation_mut() -= glam::Vec4::ONE * (glam::Vec4::Z * *delta as f32);

        //todo

        self.boundary
            .cross(&mut self.sync_barrier, |section, storage| {
                storage.bind_shader_storage(section as usize);
                // self.command_queue.call();
            });
    }
}

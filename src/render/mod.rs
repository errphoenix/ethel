pub mod buffer;
pub mod command;
pub mod sync;

use crate::{
    RenderHandler,
    mesh::Meshadata,
    render::{buffer::ImmutableBuffer, sync::SyncBarrier},
    state::{
        camera::ViewPoint,
        cross::{Consumer, Cross},
    },
};

pub trait GlPropertyEnum {
    fn as_gl_enum(&self) -> u32;
}

const ORTHO_NEAR: f32 = 0.0;
const ORTHO_FAR: f32 = 2.0;
const PERSP_NEAR: f32 = 0.1;

pub fn projection_orthographic(width: f32, height: f32) -> glam::Mat4 {
    glam::Mat4::orthographic_rh_gl(0.0, width, height, 0.0, ORTHO_NEAR, ORTHO_FAR)
}

pub fn projection_perspective(width: f32, height: f32, fov_degrees: f32) -> glam::Mat4 {
    glam::Mat4::perspective_infinite_reverse_rh(
        fov_degrees.to_radians(),
        width / height,
        PERSP_NEAR,
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Resolution {
    dirty: bool,
    pub width: f32,
    pub height: f32,
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

#[derive(Clone, Debug)]
pub struct ScreenSpace {
    resolution: Resolution,
    projection: glam::Mat4,
    fov: f32,
}

impl Default for ScreenSpace {
    fn default() -> Self {
        Self::new(Resolution::default(), Self::DEFAULT_FOV_DEG)
    }
}

impl ScreenSpace {
    pub const DEFAULT_FOV_DEG: f32 = 90.0;

    pub fn new(resolution: Resolution, fov_deg: f32) -> Self {
        let proj_mat = projection_perspective(resolution.width, resolution.height(), fov_deg);
        Self {
            resolution,
            fov: fov_deg,
            projection: proj_mat,
        }
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn fov_mut(&mut self) -> &mut f32 {
        &mut self.fov
    }

    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn resolution_mut(&mut self) -> &mut Resolution {
        &mut self.resolution
    }

    pub fn projection(&self) -> &glam::Mat4 {
        &self.projection
    }

    pub fn projection_mut(&mut self) -> &mut glam::Mat4 {
        &mut self.projection
    }
}

/// Render state for the Janus rendering Context
#[derive(Debug, Default)]
pub struct Renderer<D: Sized, T: RenderHandler<D>> {
    // only used for rendering as sometimes opengl may refuse to draw anything
    // without a vao bound during draw calls
    render_vao: u32,

    mesh_buffer: ImmutableBuffer<2>,
    metadata: Meshadata,

    screen: janus::sync::Mirror<ScreenSpace>,
    view: janus::sync::Mirror<ViewPoint>,

    pub(crate) handler: T,

    sync_barrier: SyncBarrier,
    boundary: Cross<Consumer, D>,
}

impl<D: Sized, T: RenderHandler<D>> Renderer<D, T> {
    pub fn mesh_buffer(&self) -> &ImmutableBuffer<2> {
        &self.mesh_buffer
    }

    pub fn mesh_buffer_mut(&mut self) -> &mut ImmutableBuffer<2> {
        &mut self.mesh_buffer
    }

    pub fn screen_space(&self) -> &ScreenSpace {
        &self.screen
    }

    pub fn screen_space_mirror(&self) -> &janus::sync::Mirror<ScreenSpace> {
        &self.screen
    }

    pub fn screen_space_mirror_mut(&mut self) -> &mut janus::sync::Mirror<ScreenSpace> {
        &mut self.screen
    }

    pub fn metadata(&self) -> &Meshadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Meshadata {
        &mut self.metadata
    }

    pub fn boundary(&self) -> &Cross<Consumer, D> {
        &self.boundary
    }

    pub fn boundary_mut(&mut self) -> &mut Cross<Consumer, D> {
        &mut self.boundary
    }

    pub fn view(&self) -> &ViewPoint {
        &self.view
    }

    pub fn viewpoint_mirror(&self) -> &janus::sync::Mirror<ViewPoint> {
        &self.view
    }

    pub fn viewpoint_mirror_mut(&mut self) -> &mut janus::sync::Mirror<ViewPoint> {
        &mut self.view
    }
}

impl<D: Sized, T: RenderHandler<D>> janus::context::Draw for Renderer<D, T> {
    fn draw(&mut self, dt: janus::context::DeltaTime) {
        if self.render_vao == 0 {
            unsafe {
                janus::gl::GenVertexArrays(1, &mut self.render_vao);
                janus::gl::BindVertexArray(self.render_vao);
            }
        }
        {
            if self.screen.check_sync_status() {
                self.screen.sync().unwrap();
                let resolution = self.screen.resolution;
                if resolution.is_changed() {
                    self.screen.publish_with(|screen| {
                        let fov = screen.fov();
                        let w = resolution.width;
                        let h = resolution.height;

                        screen.projection = projection_perspective(w, h, fov);
                        screen.resolution.dirty = true;
                    });

                    let w = resolution.width as i32;
                    let h = resolution.height as i32;
                    unsafe {
                        janus::gl::Viewport(0, 0, w, h);
                    }
                }
            }
        }

        self.handler.pre_frame(&mut self.screen, &mut self.view, dt);
        self.boundary
            .cross(&mut self.sync_barrier, |section, storage| {
                self.mesh_buffer.bind_shader_storage();
                self.handler.render_frame(&storage, section);
            });

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
        self.screen.publish_with(|screen| {
            screen.resolution = Resolution {
                dirty: true,
                width: w,
                height: h,
            }
        });
    }
}

impl<D: Sized, T: RenderHandler<D>> Drop for Renderer<D, T> {
    fn drop(&mut self) {
        unsafe {
            janus::gl::DeleteVertexArrays(1, &self.render_vao);
        }
    }
}

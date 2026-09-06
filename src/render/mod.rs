pub mod buffer;
pub mod command;
pub mod sync;

use std::sync::Arc;

use glam::Vec4Swizzles;

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
    ortho_proj: glam::Mat4,
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
        let proj_mat = projection_perspective(resolution.width, resolution.height, fov_deg);
        let ortho_proj = projection_orthographic(resolution.width, resolution.height);
        Self {
            resolution,
            fov: fov_deg,
            projection: proj_mat,
            ortho_proj,
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

    pub fn orto_projection(&self) -> &glam::Mat4 {
        &self.ortho_proj
    }

    pub fn ortho_projection_mut(&mut self) -> &mut glam::Mat4 {
        &mut self.ortho_proj
    }

    #[inline]
    pub const fn to_ndc(&self, screen: (f32, f32)) -> glam::Vec3 {
        let x = (2.0 * screen.0) / self.resolution.width - 1.0;
        let y = 1.0 - (2.0 * screen.1) / self.resolution.height;
        glam::vec3(x, y, 1.0)
    }

    #[inline]
    pub const fn to_clip_space(&self, screen: (f32, f32)) -> glam::Vec4 {
        let ndc = self.to_ndc(screen);
        glam::vec4(ndc.x, ndc.y, -1.0, 1.0)
    }

    #[inline]
    pub fn to_eye_space(&self, screen: (f32, f32)) -> glam::Vec4 {
        let clip = self.to_clip_space(screen);
        let inv_proj = self.projection.inverse();
        let eye_ray = inv_proj * clip;
        glam::vec4(eye_ray.x, eye_ray.y, -1.0, 0.0)
    }

    #[inline]
    pub fn to_world_space(&self, screen: (f32, f32), inverse_view: glam::Mat4) -> glam::Vec3 {
        let eye = self.to_eye_space(screen);
        let eye_world = (inverse_view * eye).xyz();
        eye_world.normalize()
    }
}

#[derive(Debug, Default)]
pub struct MeshBuffers {
    pub statics: ImmutableBuffer<2>,
    pub triangles: ImmutableBuffer<1>,
}
impl MeshBuffers {
    /// Bind static geometry data to a ssbo at index `ssbo_index`
    /// (or index 10 if absent).
    ///
    /// The metadata and vertex arrays are bound to a single ssbo as 2 fixed-len
    /// arrays. The length of the arrays is defined in the [`layout`] passed
    /// during initialization in [`StartupHandler::with_mesh_layouts`]
    ///
    /// [`StartupHandler::with_mesh_layouts`]: crate::StartupHandler::with_mesh_layouts
    /// [`layout`]: crate::render::buffer::layout::Layout
    pub fn bind_ssbo_statics(&self, ssbo_index: Option<u32>) {
        self.statics.bind_shader_storage_arrays(0, 2, ssbo_index);
    }

    /// Bind mesh vertices data to a ssbo at index `ssbo_index`
    /// (or index 10 if absent).
    ///
    /// The vertex array is bound to a single ssbo as a runtime array, there
    /// are no specific length requirements for the glsl ssbo block.
    pub fn bind_ssbo_vertices(&self, ssbo_index: Option<u32>) {
        self.statics.bind_shader_storage_single(0, ssbo_index);
    }

    /// Bind mesh metadata to a ssbo at index `ssbo_index`
    /// (or index 10 if absent).
    ///
    /// The metadata array is bound to a single ssbo as a runtime array, there
    /// are no specific length requirements for the glsl ssbo block.
    pub fn bind_ssbo_metadata(&self, ssbo_index: Option<u32>) {
        self.statics.bind_shader_storage_single(1, ssbo_index);
    }

    /// Bind mesh triangle data to a ssbo at index `ssbo_index`
    /// (or index 11 if absent).
    ///
    /// The triangle array is bound to a single ssbo as a runtime array, there
    /// are no specific length requirements for the glsl ssbo block.
    pub fn bind_ssbo_triangles(&self, ssbo_index: Option<u32>) {
        self.triangles.bind_shader_storage_single(0, ssbo_index);
    }

    pub const fn statics(&self) -> &ImmutableBuffer<2> {
        &self.statics
    }

    pub const fn triangles(&self) -> &ImmutableBuffer<1> {
        &self.triangles
    }
}

/// Render state for the Janus rendering Context
#[derive(Debug, Default)]
pub struct Renderer<D: Sized, T: RenderHandler<D>> {
    // only used for rendering as sometimes opengl may refuse to draw anything
    // without a vao bound during draw calls
    pub(crate) internal_vao: u32,

    pub mesh_buffers: MeshBuffers,
    pub metadata: Meshadata,

    pub screen_space: janus::sync::Mirror<ScreenSpace>,
    pub viewpoint: Arc<janus::sync::TriCell<ViewPoint>>,

    pub(crate) handler: T,

    sync_barrier: SyncBarrier,
    pub boundary: Cross<Consumer, D>,
}

impl<D: Sized, T: RenderHandler<D>> Renderer<D, T> {
    /// Returns the internal OpenGL VAO.
    ///
    /// This is only used for rendering as some OpenGL drivers may refuse to
    /// draw anything without a VAO bound during draw calls.
    ///
    /// The application may use this existing VAO for additional functions,
    /// such as indexed drawing.
    pub const fn internal_vao(&self) -> u32 {
        self.internal_vao
    }

    pub fn handler_init_callback<F: FnOnce(&mut T)>(&mut self, callback: F) {
        callback(&mut self.handler)
    }

    pub const fn mesh_buffers(&self) -> &MeshBuffers {
        &self.mesh_buffers
    }

    pub const fn screen_space(&self) -> &ScreenSpace {
        self.screen_space.get()
    }

    pub const fn screen_space_mirror(&self) -> &janus::sync::Mirror<ScreenSpace> {
        &self.screen_space
    }

    pub const fn metadata(&self) -> &Meshadata {
        &self.metadata
    }

    pub const fn boundary(&self) -> &Cross<Consumer, D> {
        &self.boundary
    }

    pub fn view(&self) -> &ViewPoint {
        &self.viewpoint
    }

    pub const fn viewpoint_shared(&self) -> &Arc<janus::sync::TriCell<ViewPoint>> {
        &self.viewpoint
    }
}
impl<D: Sized, T: RenderHandler<D>> janus::context::Draw for Renderer<D, T> {
    fn draw(&mut self, dt: janus::context::DeltaTime) {
        let mut res_has_changed = false;
        {
            if self.screen_space.check_sync_status() {
                self.screen_space.sync().unwrap();
                let resolution = self.screen_space.resolution;
                if resolution.is_changed() {
                    res_has_changed = true;
                    self.screen_space.publish_with(|screen| {
                        let fov = screen.fov();
                        let w = resolution.width;
                        let h = resolution.height;

                        screen.projection = projection_perspective(w, h, fov);
                        screen.ortho_proj = projection_orthographic(w, h);
                    });

                    let w = resolution.width as i32;
                    let h = resolution.height as i32;
                    unsafe {
                        janus::gl::Viewport(0, 0, w, h);
                    }
                }
            }
        }

        self.handler
            .pre_frame(&mut self.screen_space, &self.viewpoint, dt);
        self.boundary
            .cross(&mut self.sync_barrier, |section, storage| {
                self.mesh_buffers.bind_ssbo_statics(None);
                self.mesh_buffers.bind_ssbo_triangles(None);
                self.handler.render_frame(&storage, section);
            });

        if res_has_changed {
            self.screen_space
                .publish_with(|s| s.resolution.dirty = false);
        }

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
        self.screen_space.publish_with(|screen| {
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
        if self.internal_vao != 0 {
            unsafe {
                janus::gl::DeleteVertexArrays(1, &self.internal_vao);
            }
        }
    }
}

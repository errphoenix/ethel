pub mod data;

use glam::Mat4;

use crate::mesh::Meshadata;

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
pub struct Context {
    resolution: Resolution,
    metadata: Meshadata,
}

impl janus::context::Draw for Context {
    fn draw(&self, _delta: janus::context::DeltaTime) {
        let _proj_ortho = projection_orthographic(self.resolution.width, self.resolution.height);

        todo!()
    }
}

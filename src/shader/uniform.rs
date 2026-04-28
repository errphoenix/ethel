use crate::shader::{UniformLocation, glsl::Glsl};

pub trait UploadUniform: Glsl {
    fn upload(&self, location: UniformLocation);
}

impl UploadUniform for glam::Vec2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform2f(*location, self.x, self.y);
        }
    }
}

impl UploadUniform for glam::Vec3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform3f(*location, self.x, self.y, self.z);
        }
    }
}

impl UploadUniform for glam::Vec4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform4f(*location, self.x, self.y, self.z, self.w);
        }
    }
}

impl UploadUniform for glam::Mat2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::UniformMatrix2fv(
                *location,
                1,
                janus::gl::FALSE,
                self.to_cols_array().as_ptr(),
            );
        }
    }
}

impl UploadUniform for glam::Mat3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::UniformMatrix3fv(
                *location,
                1,
                janus::gl::FALSE,
                self.to_cols_array().as_ptr(),
            );
        }
    }
}

impl UploadUniform for glam::Mat4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::UniformMatrix4fv(
                *location,
                1,
                janus::gl::FALSE,
                self.to_cols_array().as_ptr(),
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlslUniform(&'static str);

impl GlslUniform {
    pub const fn new(string: &'static str) -> Self {
        Self(string)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for GlslUniform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl super::Inject for GlslUniform {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{self}")
    }
}

#[macro_export]
macro_rules! shader_glsl_uniform {
    ($gl_name:ident: $gl_type:ident) => {
        GlslUniform::new(concat!(
            "uniform ",
            stringify!($gl_type),
            " ",
            stringify!($gl_name),
            ";\n"
        ))
    };
}

#[macro_export]
macro_rules! shader_glsl_build_uniform_interface {
    ($gl_name:ident: $gl_type:ident => $r_type:ty) => {
        paste::paste! {
            pub fn [< uniform_ $gl_name _ $gl_type >] (&self, $gl_name: $r_type) {
                let location = self.[< location_ $gl_name _ $gl_type >];
                $crate::shader::uniform::UploadUniform::upload(&$gl_name, location);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_compose_glsl_uniform() {
        const TEST: &str = "uniform mat4 projection;\n";
        let uniform = shader_glsl_uniform!(projection: mat4);
        assert_eq!(TEST, uniform.as_str());
    }
}

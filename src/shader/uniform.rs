use crate::shader::{
    UniformLocation,
    glsl::{Glsl, GlslAlloc},
};

#[macro_export]
macro_rules! shader_glsl_build_uniform_interface {
    ($gl_name:ident: $gl_type:expr => $r_type:ty; $up_l:block) => {
        paste:paste! {
            pub fn [< uniform_ $gl_name _ $gl_type]>(&self, $gl_name: $r_type) $up_l
        }
    };
    ($uni:expr) => {

    }
}

pub trait HasUniform: Glsl {
    fn upload(&self, location: UniformLocation);
}

impl HasUniform for glam::Vec2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform2f(*location, self.x, self.y);
        }
    }
}

impl HasUniform for glam::Vec3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform3f(*location, self.x, self.y, self.z);
        }
    }
}

impl HasUniform for glam::Vec4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform4f(*location, self.x, self.y, self.z, self.w);
        }
    }
}

impl HasUniform for glam::Mat2 {
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

impl HasUniform for glam::Mat3 {
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

impl HasUniform for glam::Mat4 {
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
pub struct ShaderUniform<T: HasUniform> {
    name: &'static str,
    _type: std::marker::PhantomData<T>,
}

impl<T: HasUniform> ShaderUniform<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _type: std::marker::PhantomData,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl<T: HasUniform> GlslAlloc for ShaderUniform<T> {
    fn to_glsl_alloc(&self) -> String {
        format!("uniform {} {};", T::to_glsl(), self.name)
    }
}

impl<T: HasUniform> super::Inject for ShaderUniform<T> {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{}", self.to_glsl_alloc())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_compose_glsl_uniform() {
        const TEST: &str = "uniform mat4 projection;";
        let uniform = ShaderUniform::<glam::Mat4>::new("projection").to_glsl_alloc();
        assert_eq!(TEST, &uniform);
    }
}

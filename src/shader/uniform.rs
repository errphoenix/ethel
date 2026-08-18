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
impl UploadUniform for glam::IVec2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform2i(*location, self.x, self.y);
        }
    }
}
impl UploadUniform for glam::IVec3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform3i(*location, self.x, self.y, self.z);
        }
    }
}
impl UploadUniform for glam::IVec4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform4i(*location, self.x, self.y, self.z, self.w);
        }
    }
}
impl UploadUniform for glam::UVec2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform2ui(*location, self.x, self.y);
        }
    }
}
impl UploadUniform for glam::UVec3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform3ui(*location, self.x, self.y, self.z);
        }
    }
}
impl UploadUniform for glam::UVec4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform4ui(*location, self.x, self.y, self.z, self.w);
        }
    }
}
impl UploadUniform for glam::BVec2 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform2i(*location, self.x as i32, self.y as i32);
        }
    }
}
impl UploadUniform for glam::BVec3 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform3i(*location, self.x as i32, self.y as i32, self.z as i32);
        }
    }
}
impl UploadUniform for glam::BVec4 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform4i(
                *location,
                self.x as i32,
                self.y as i32,
                self.z as i32,
                self.w as i32,
            );
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
impl UploadUniform for f32 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform1f(*location, *self);
        }
    }
}
impl UploadUniform for u32 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform1ui(*location, *self);
        }
    }
}
impl UploadUniform for i32 {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            janus::gl::Uniform1i(*location, *self);
        }
    }
}
impl UploadUniform for bool {
    fn upload(&self, location: UniformLocation) {
        UploadUniform::upload(&(*self as u32), location);
    }
}
impl<const SIZE: usize> UploadUniform for [bool; SIZE] {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            match SIZE {
                0 => unreachable!(),
                1 => UploadUniform::upload(&self[0], location),
                2 => janus::gl::Uniform2i(*location, self[0] as i32, self[1] as i32),
                3 => {
                    janus::gl::Uniform3i(*location, self[0] as i32, self[1] as i32, self[2] as i32)
                }
                4 => janus::gl::Uniform4i(
                    *location,
                    self[0] as i32,
                    self[1] as i32,
                    self[2] as i32,
                    self[3] as i32,
                ),
                _ => janus::gl::Uniform1iv(*location, SIZE as i32, self.as_ptr().cast()),
            }
        }
    }
}
impl<const SIZE: usize> UploadUniform for [f32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            match SIZE {
                0 => unreachable!(),
                1 => UploadUniform::upload(&self[0], location),
                2 => janus::gl::Uniform2f(*location, self[0], self[1]),
                3 => janus::gl::Uniform3f(*location, self[0], self[1], self[2]),
                4 => janus::gl::Uniform4f(*location, self[0], self[1], self[2], self[3]),
                _ => janus::gl::Uniform1fv(*location, SIZE as i32, self.as_ptr()),
            }
        }
    }
}
impl<const SIZE: usize> UploadUniform for [u32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            match SIZE {
                0 => unreachable!(),
                1 => UploadUniform::upload(&self[0], location),
                2 => janus::gl::Uniform2ui(*location, self[0], self[1]),
                3 => janus::gl::Uniform3ui(*location, self[0], self[1], self[2]),
                4 => janus::gl::Uniform4ui(*location, self[0], self[1], self[2], self[3]),
                _ => janus::gl::Uniform1uiv(*location, SIZE as i32, self.as_ptr()),
            }
        }
    }
}
impl<const SIZE: usize> UploadUniform for [i32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        unsafe {
            match SIZE {
                0 => unreachable!(),
                1 => UploadUniform::upload(&self[0], location),
                2 => janus::gl::Uniform2i(*location, self[0], self[1]),
                3 => janus::gl::Uniform3i(*location, self[0], self[1], self[2]),
                4 => janus::gl::Uniform4i(*location, self[0], self[1], self[2], self[3]),
                _ => janus::gl::Uniform1iv(*location, SIZE as i32, self.as_ptr()),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GlslUniform(String);

impl GlslUniform {
    pub const fn new(string: String) -> Self {
        Self(string)
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
    ($($arr_n:literal,)? $gl_name:ident: $gl_type:ident) => {
        $crate::shader::uniform::GlslUniform::new(concat!(
            "uniform ",
            stringify!($gl_type),
            " ",
            stringify!($gl_name),
            $("[", $arr_n, "]",)?
            ";\n"
        ).to_string())
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
    (array $ac:literal, $gl_name:ident: $gl_type:ident => $r_type:ty) => {
        paste::paste! {
            pub fn [< uniform_ $gl_name _ $gl_type v >] (&self, $gl_name: [$r_type; $ac]) {
                let location = self.[< location_ $gl_name _ $gl_type >];
                for i in 0..$ac {
                    let location = $crate::shader::UniformLocation(location.0 + i);
                    $crate::shader::uniform::UploadUniform::upload(&$gl_name[i as usize], location);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! shader_glsl_internal_image {
    (on $idx:expr $(, for $len:expr)? => $name:ident : $image_type:ident as $format:ident $($m:ident)* ) => {
        {
            #[allow(unused)]
            let mut pfx = $crate::shader_glsl_internal_image!(@prefix $idx, $format);
            $($crate::shader_glsl_internal_image!(@parse pfx $m);)*
            let sfx = $crate::shader_glsl_internal_image!(@suffix $name, $image_type, $($len)?);
            $crate::shader::uniform::GlslUniform::new(format!("{pfx} {sfx}"))
        }
    };

    (@prefix $idx:expr, $format: ident) => {
        format!(
            "layout(binding = {}, {})", $idx, stringify!($format)
        )
    };
    (@suffix $name:ident, $type:ident, $($len:expr)?) => {
        concat!(
            "uniform ", stringify!($type), " ", stringify!($name),
            $("[", $len, "]",)?
            ";"
        )
    };

    (@parse $pfx:ident writeonly) => { $pfx = format!("{} writeonly", $pfx) };
    (@parse $pfx:ident readonly ) => { $pfx = format!("{} readonly" , $pfx) };
    (@parse $pfx:ident coherent ) => { $pfx = format!("{} coherent" , $pfx) };
    (@parse $pfx:ident volatile ) => { $pfx = format!("{} volatile" , $pfx) };
    (@parse $pfx:ident restrict ) => { $pfx = format!("{} restrict" , $pfx) };
    (@parse ) => {};
}

#[cfg(test)]
mod tests {
    #[test]
    fn shader_compose_glsl_uniform() {
        const TEST: &str = "uniform mat4 projection;\n";
        let uniform = shader_glsl_uniform!(projection: mat4);
        assert_eq!(TEST, uniform.as_str());
    }

    #[test]
    fn shader_compose_glsl_image() {
        const B: u32 = 1;
        const TEST_A: &str = "layout(binding = 1, rgba16f) uniform imageCube env_map[4];";
        let image = shader_glsl_internal_image!(on B, for 4 => env_map : imageCube as rgba16f);
        assert_eq!(TEST_A, image.as_str());

        const TEST_B: &str =
            "layout(binding = 5, rgba8) readonly restrict uniform image2D im_imag;";
        let image =
            shader_glsl_internal_image!(on 5 => im_imag : image2D as rgba8 readonly restrict);
        assert_eq!(TEST_B, image.as_str());
    }
}

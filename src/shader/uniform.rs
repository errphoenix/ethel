use crate::shader::{ShaderProgram, UniformLocation, glsl::Glsl};

pub trait UploadUniform: Glsl {
    fn upload(&self, location: UniformLocation);

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation);
}

#[macro_export]
macro_rules! type_uniform_interface {
    ($t:ty : fn $fn:ident => Self $(as $ct:tt)?) => {
        impl $crate::shader::uniform::UploadUniform for $t {
            fn upload(&self, location: $crate::shader::UniformLocation) {
                unsafe {
                    paste::paste! {
                        janus::gl::[< $fn >](
                            *location,
                            *self $(as $ct)?
                        );
                    }
                }
            }
            fn upload_direct(&self, program: &impl $crate::shader::ShaderProgram, location: $crate::shader::UniformLocation) {
                unsafe {
                    paste::paste! {
                        janus::gl::[< Program $fn >](
                            program.shader_program(),
                            *location,
                            *self $(as $ct)?
                        );
                    }
                }
            }
        }
    };
    ($t:ty : fn $fn:ident => $($fld:tt $(as $ct:tt)?),+ $(,)?) => {
        impl $crate::shader::uniform::UploadUniform for $t {
            fn upload(&self, location: $crate::shader::UniformLocation) {
                unsafe {
                    paste::paste! {
                        janus::gl::[< $fn >](
                            *location
                            $(, self.$fld $(as $ct)?)+
                        );
                    }
                }
            }
            fn upload_direct(&self, program: &impl $crate::shader::ShaderProgram, location: $crate::shader::UniformLocation) {
                unsafe {
                    paste::paste! {
                        janus::gl::[< Program $fn >](
                            program.shader_program(),
                            *location
                            $(, self.$fld $(as $ct)?)+
                        );
                    }
                }
            }
        }
    };
}

type_uniform_interface!(glam::Vec2  : fn Uniform2f  => x, y);
type_uniform_interface!(glam::Vec3  : fn Uniform3f  => x, y, z);
type_uniform_interface!(glam::Vec4  : fn Uniform4f  => x, y, z, w);
type_uniform_interface!(glam::IVec2 : fn Uniform2i  => x, y);
type_uniform_interface!(glam::IVec3 : fn Uniform3i  => x, y, z);
type_uniform_interface!(glam::IVec4 : fn Uniform4i  => x, y, z, w);
type_uniform_interface!(glam::UVec2 : fn Uniform2ui => x, y);
type_uniform_interface!(glam::UVec3 : fn Uniform3ui => x, y, z);
type_uniform_interface!(glam::UVec4 : fn Uniform4ui => x, y, z, w);
type_uniform_interface!(glam::BVec2 : fn Uniform2i  => x as i32, y as i32);
type_uniform_interface!(glam::BVec3 : fn Uniform3i  => x as i32, y as i32, z as i32);
type_uniform_interface!(glam::BVec4 : fn Uniform4i  => x as i32, y as i32, z as i32, w as i32);
type_uniform_interface!(f32  : fn Uniform1f  => Self);
type_uniform_interface!(i32  : fn Uniform1i  => Self);
type_uniform_interface!(u32  : fn Uniform1ui => Self);
type_uniform_interface!(bool : fn Uniform1i  => Self as i32);

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

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        unsafe {
            janus::gl::ProgramUniformMatrix2fv(
                program.shader_program(),
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

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        unsafe {
            janus::gl::ProgramUniformMatrix3fv(
                program.shader_program(),
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

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        unsafe {
            janus::gl::ProgramUniformMatrix4fv(
                program.shader_program(),
                *location,
                1,
                janus::gl::FALSE,
                self.to_cols_array().as_ptr(),
            );
        }
    }
}

impl<const SIZE: usize> UploadUniform for [u32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::Uniform1ui(*location, self[0]);
            },
            2 => unsafe {
                janus::gl::Uniform2ui(*location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::Uniform3ui(*location, self[0], self[1], self[2]);
            },
            4 => unsafe {
                janus::gl::Uniform4ui(*location, self[0], self[1], self[2], self[4]);
            },
            _ => unsafe {
                janus::gl::Uniform1uiv(*location, SIZE as i32, self.as_ptr().cast());
            },
        }
    }

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::ProgramUniform1ui(program.shader_program(), *location, self[0]);
            },
            2 => unsafe {
                janus::gl::ProgramUniform2ui(program.shader_program(), *location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::ProgramUniform3ui(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                );
            },
            4 => unsafe {
                janus::gl::ProgramUniform4ui(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                    self[4],
                );
            },
            _ => unsafe {
                janus::gl::ProgramUniform1uiv(
                    program.shader_program(),
                    *location,
                    SIZE as i32,
                    self.as_ptr().cast(),
                );
            },
        }
    }
}
impl<const SIZE: usize> UploadUniform for [i32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::Uniform1i(*location, self[0]);
            },
            2 => unsafe {
                janus::gl::Uniform2i(*location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::Uniform3i(*location, self[0], self[1], self[2]);
            },
            4 => unsafe {
                janus::gl::Uniform4i(*location, self[0], self[1], self[2], self[4]);
            },
            _ => unsafe {
                janus::gl::Uniform1iv(*location, SIZE as i32, self.as_ptr().cast());
            },
        }
    }

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::ProgramUniform1i(program.shader_program(), *location, self[0]);
            },
            2 => unsafe {
                janus::gl::ProgramUniform2i(program.shader_program(), *location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::ProgramUniform3i(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                );
            },
            4 => unsafe {
                janus::gl::ProgramUniform4i(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                    self[4],
                );
            },
            _ => unsafe {
                janus::gl::ProgramUniform1iv(
                    program.shader_program(),
                    *location,
                    SIZE as i32,
                    self.as_ptr().cast(),
                );
            },
        }
    }
}
impl<const SIZE: usize> UploadUniform for [bool; SIZE] {
    fn upload(&self, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::Uniform1i(*location, self[0] as i32);
            },
            2 => unsafe {
                janus::gl::Uniform2i(*location, self[0] as i32, self[1] as i32);
            },
            3 => unsafe {
                janus::gl::Uniform3i(*location, self[0] as i32, self[1] as i32, self[2] as i32);
            },
            4 => unsafe {
                janus::gl::Uniform4i(
                    *location,
                    self[0] as i32,
                    self[1] as i32,
                    self[2] as i32,
                    self[4] as i32,
                );
            },
            _ => unsafe {
                janus::gl::Uniform1iv(*location, SIZE as i32, self.as_ptr().cast());
            },
        }
    }

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::ProgramUniform1i(program.shader_program(), *location, self[0] as i32);
            },
            2 => unsafe {
                janus::gl::ProgramUniform2i(
                    program.shader_program(),
                    *location,
                    self[0] as i32,
                    self[1] as i32,
                );
            },
            3 => unsafe {
                janus::gl::ProgramUniform3i(
                    program.shader_program(),
                    *location,
                    self[0] as i32,
                    self[1] as i32,
                    self[2] as i32,
                );
            },
            4 => unsafe {
                janus::gl::ProgramUniform4i(
                    program.shader_program(),
                    *location,
                    self[0] as i32,
                    self[1] as i32,
                    self[2] as i32,
                    self[4] as i32,
                );
            },
            _ => unsafe {
                janus::gl::ProgramUniform1iv(
                    program.shader_program(),
                    *location,
                    SIZE as i32,
                    self.as_ptr().cast(),
                );
            },
        }
    }
}
impl<const SIZE: usize> UploadUniform for [f32; SIZE] {
    fn upload(&self, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::Uniform1f(*location, self[0]);
            },
            2 => unsafe {
                janus::gl::Uniform2f(*location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::Uniform3f(*location, self[0], self[1], self[2]);
            },
            4 => unsafe {
                janus::gl::Uniform4f(*location, self[0], self[1], self[2], self[4]);
            },
            12 => unsafe {
                janus::gl::UniformMatrix3fv(*location, 1, janus::gl::FALSE, self.as_ptr().cast());
            },
            16 => unsafe {
                janus::gl::UniformMatrix4fv(*location, 1, janus::gl::FALSE, self.as_ptr().cast());
            },
            _ => unsafe {
                janus::gl::Uniform1fv(*location, SIZE as i32, self.as_ptr().cast());
            },
        }
    }

    fn upload_direct(&self, program: &impl ShaderProgram, location: UniformLocation) {
        match SIZE {
            0 => unreachable!(),
            1 => unsafe {
                janus::gl::ProgramUniform1f(program.shader_program(), *location, self[0]);
            },
            2 => unsafe {
                janus::gl::ProgramUniform2f(program.shader_program(), *location, self[0], self[1]);
            },
            3 => unsafe {
                janus::gl::ProgramUniform3f(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                );
            },
            4 => unsafe {
                janus::gl::ProgramUniform4f(
                    program.shader_program(),
                    *location,
                    self[0],
                    self[1],
                    self[2],
                    self[4],
                );
            },
            12 => unsafe {
                janus::gl::ProgramUniformMatrix3fv(
                    program.shader_program(),
                    *location,
                    1,
                    janus::gl::FALSE,
                    self.as_ptr().cast(),
                );
            },
            16 => unsafe {
                janus::gl::ProgramUniformMatrix4fv(
                    program.shader_program(),
                    *location,
                    1,
                    janus::gl::FALSE,
                    self.as_ptr().cast(),
                );
            },
            _ => unsafe {
                janus::gl::ProgramUniform1fv(
                    program.shader_program(),
                    *location,
                    SIZE as i32,
                    self.as_ptr().cast(),
                );
            },
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
                $crate::shader::uniform::UploadUniform::upload_direct(&$gl_name, self.handle(), location);
            }
        }
    };
    (array $ac:literal, $gl_name:ident: $gl_type:ident => $r_type:ty) => {
        paste::paste! {
            pub fn [< uniform_ $gl_name _ $gl_type v >] (&self, $gl_name: [$r_type; $ac]) {
                let location = self.[< location_ $gl_name _ $gl_type >];
                for i in 0..$ac {
                    let location = $crate::shader::UniformLocation(location.0 + i);
                    $crate::shader::uniform::UploadUniform::upload_direct(&$gl_name[i as usize], self.handle(), location);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! shader_glsl_internal_sampler {
    (on $s_idx:expr $(, for $s_len:expr)? => $us_name:ident : $sampler_type:ident ; $unit_offset:ident) => {
        {
            assert!(
                $s_idx >= $unit_offset,
                "conflicting sampler binding index: current offset is {} but specified binding {}",
                $unit_offset, $s_idx
            );

            #[allow(unused)]
            {
                $unit_offset += 1;
                $(
                    $unit_offset += $s_len - 1;
                )?
            }

            let name = concat!(
                stringify!($us_name),
                $("[", $s_len, "]",)?
            );
            $crate::shader::uniform::GlslUniform::new(
                format!(
                    "layout(binding = {}) uniform {} {name};",
                    $s_idx, stringify!($sampler_type)
                )
            )
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
    fn shader_compose_glsl_sampler() {
        let mut uo = 0;
        let sampler0 =
            shader_glsl_internal_sampler!(on 0, for 16 => material_map : sampler2DArray ; uo);
        let sampler1 = shader_glsl_internal_sampler!(on 16 => some_sampler : sampler2D; uo);

        const S0: &str = "layout(binding = 0) uniform sampler2DArray material_map[16];";
        const S1: &str = "layout(binding = 16) uniform sampler2D some_sampler;";

        assert_eq!(sampler0.as_str(), S0);
        assert_eq!(sampler1.as_str(), S1);
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

pub mod glsl;
pub mod uniform;

pub use crate::shader_glsl_ssbo;

#[allow(unused_imports)]
pub use glsl::{GlslHeap, GlslStack};

use std::{hash::Hash, io::BufRead};

use janus::gl;
use tracing::{Level, event};

use crate::shader::{
    glsl::{GlslAlloc, GlslType, ShadingVersion},
    uniform::GlslUniform,
};

pub trait WriteValue {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result;
}

pub trait Inject {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result;
}

pub trait ShaderHeader: Inject {}

pub trait ShaderBody: Inject {}

#[derive(Clone, Debug)]
pub struct Constant<T: Clone + Copy + WriteValue> {
    name: String,
    value: T,
}

impl<T: Clone + Copy + WriteValue> Constant<T> {
    pub fn new(name: &str, value: T) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn value(&self) -> T {
        self.value
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Default, Debug)]
pub struct UniformLocation(i32);

impl std::fmt::Display for UniformLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl UniformLocation {
    pub fn get(&self) -> i32 {
        self.0
    }
}

impl std::ops::Deref for UniformLocation {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ShaderComposer {
    version: ShadingVersion,
    header: String,
    uniforms_section: String,
    body: String,
    source: String,
}

impl ShaderComposer {
    pub fn new(version: ShadingVersion) -> Self {
        Self {
            version,
            header: String::new(),
            uniforms_section: String::new(),
            body: String::new(),
            source: String::new(),
        }
    }

    pub fn version(&self) -> ShadingVersion {
        self.version
    }

    pub fn header(&self) -> &str {
        &self.header
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Inject shader SSBO declarations, struct type declarations, and
    /// attributes.
    pub fn inject_header(&mut self, header: &impl ShaderHeader) -> std::fmt::Result {
        header.inject_shader(&mut self.header)
    }

    /// Inject shader utility (or "library") source code, available for use in
    /// the shader's main function.
    pub fn inject_body(&mut self, body: &impl ShaderBody) -> std::fmt::Result {
        body.inject_shader(&mut self.body)
    }

    /// Set the shader's main function contents.
    pub fn set_source(&mut self, source: impl Into<ShaderSource>) {
        self.source = source.into().0;
    }

    /// Add a constant variable to the shader's header.
    ///
    /// Compatible with Rust types. See [`Constant`].
    pub fn add_constant<T: WriteValue + Clone + Copy + GlslType>(
        &mut self,
        constant: &Constant<T>,
    ) {
        let glsl = constant.to_glsl_alloc();
        self.header += &glsl;
        self.header += "\n";
    }

    /// Add a uniform declaration to the shader's body.
    pub fn add_uniform(&mut self, uniform: GlslUniform) -> std::fmt::Result {
        uniform.inject_shader(&mut self.uniforms_section)
    }
}

/// The raw source code of a shader's `main()` function.
#[derive(Clone, Debug, Default)]
pub struct ShaderSource(String);

impl std::fmt::Display for ShaderSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ShaderSource {
    fn from(value: String) -> Self {
        ShaderSource::new(&value)
    }
}

impl From<&str> for ShaderSource {
    fn from(value: &str) -> Self {
        ShaderSource::new(value)
    }
}

impl ShaderSource {
    pub fn new(source: &str) -> Self {
        Self(format!("void main() {{\n {source} \n}}\n"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[macro_export]
macro_rules! shader_glsl {
    (
        struct $name:ident > [$ver:expr] {
            $(attribs {
                $(
                    $attrib_glsl:expr
                )+
            };)?
            $(uniform {
                $(
                    $u_gl_name:ident: $u_gl_type:ident => $u_r_type:ty;
                )+
            };)?
            $(type {
                $(
                    $type_glsl:expr
                )+
            };)?
            $(ssbo {
                $(
                    $ssbo_glsl:expr
                )+
            };)?
            $(const {
                $(
                    $const_a:expr
                )+
            };)?
            $(lib {
                $(
                    $lib:expr;
                )+
            };)?

            src() $src:literal
        }
    ) => {
        paste::paste! {
            #[derive(Clone, Default)]
            pub struct [< Shader $name >] {
                handle: $crate::shader::ShaderHandle,

                $(
                    $(
                        [< location_ $u_gl_name _ $u_gl_type >]: $crate::shader::UniformLocation,
                    )+
                )?
            }

            impl [< Shader $name >] {
                $(
                    $(
                        $crate::shader_glsl_build_uniform_interface! {
                            $u_gl_name: $u_gl_type => $u_r_type
                        }
                    )+
                )?

                pub fn compose() -> $crate::shader::ShaderComposer {
                    let version = $crate::shader::ShadingVersion::core($ver);

                    let mut composer = $crate::shader::ShaderComposer::new(version);

                    $(
                        $(
                            composer.inject_header(&$attrib_glsl);
                        )+
                    )?

                    $(
                        $(
                            composer.add_uniform($crate::shader_glsl_uniform!($u_gl_name: $u_gl_type));
                        )+
                    )?

                    $(
                        $(
                            composer.inject_header(&$type_glsl);
                        )+
                    )?

                    $(
                        $(
                            composer.inject_header(&$ssbo_glsl);
                        )+
                    )?

                    $(
                        $(
                            composer.add_constant(&$const_a);
                        )+
                    )?

                    $(
                        $(
                            composer.inject_body(&$lib);
                        )+
                    )?

                    composer.set_source(indoc::indoc! { $src });

                    composer
                }
            }
        }
    };
}

shader_glsl! {
    struct Test > [460] {
        uniform {
            projection: mat4 => glam::Mat4;
        };

        ssbo {
            shader_glsl_ssbo! {
                buf POD_Test1 on 2 => {
                    [dyn_array vec4: pod_test_1]
                }
            }

            shader_glsl_ssbo! {
                buf POD_Test2 on 3 => {
                    [dyn_array vec4: pod_test_2]
                }
            }
        };

        const {
            Constant::new("test", 0.5)
        };

        src() "
            gl_Position = vec4(1.0);
        "
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShaderHandle {
    prog_obj: u32,
}

impl ShaderHandle {
    pub fn new(vertex: &mut impl BufRead, fragment: &mut impl BufRead) -> Self {
        let vsh = unsafe { gl::CreateShader(gl::VERTEX_SHADER) };
        let fsh = unsafe { gl::CreateShader(gl::FRAGMENT_SHADER) };
        {
            let mut v_src = String::new();
            vertex
                .read_to_string(&mut v_src)
                .expect("failed to read vertex shader source");
            let v_c_str = std::ffi::CString::new(v_src)
                .expect("unexpected null byte in vertex shader source");

            let mut f_src = String::new();
            fragment
                .read_to_string(&mut f_src)
                .expect("unexpected null byte in fragment shader source");
            let f_c_str = std::ffi::CString::new(f_src).expect("Null byte in fsh");

            unsafe {
                gl::ShaderSource(vsh, 1, &v_c_str.as_ptr(), std::ptr::null());
                gl::CompileShader(vsh);
                check_compile_status(vsh);

                gl::ShaderSource(fsh, 1, &f_c_str.as_ptr(), std::ptr::null());
                gl::CompileShader(fsh);
                check_compile_status(fsh);
            }
        }

        let program = unsafe {
            let program = gl::CreateProgram();

            gl::AttachShader(program, vsh);
            gl::AttachShader(program, fsh);
            gl::LinkProgram(program);
            check_link_status(program);

            gl::DeleteShader(vsh);
            gl::DeleteShader(fsh);

            program
        };

        Self { prog_obj: program }
    }

    pub fn uniform_location(&self, name: &str) -> UniformLocation {
        // todo: cache uniform locations
        let c_name = std::ffi::CString::new(name).unwrap();
        UniformLocation(unsafe { gl::GetUniformLocation(self.prog_obj, c_name.as_ptr()) })
    }

    pub fn uniform_mat4_glam(&self, uniform: &str, mat: glam::Mat4) {
        self.uniform_mat4_array(uniform, mat.to_cols_array());
    }

    pub fn uniform_mat4_array(&self, uniform: &str, mat: [f32; 16]) {
        let location = self.uniform_location(uniform);
        unsafe {
            gl::UniformMatrix4fv(*location, 1, gl::FALSE, mat.as_ptr());
        }
    }

    pub fn uniform_vec3_array(&self, uniform: &str, vec3: [f32; 3]) {
        let location = self.uniform_location(uniform);
        unsafe {
            gl::Uniform3f(*location, vec3[0], vec3[1], vec3[2]);
        }
    }

    pub fn uniform_vec3_glam(&self, uniform: &str, vec3: glam::Vec3) {
        let location = self.uniform_location(uniform);
        unsafe {
            gl::Uniform3f(*location, vec3[0], vec3.y, vec3.z);
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.prog_obj);
        }
    }

    pub fn unbind() {
        self::unbind();
    }
}

impl Drop for ShaderHandle {
    fn drop(&mut self) {
        if self.prog_obj == 0 {
            return;
        }
        unsafe { gl::DeleteProgram(self.prog_obj) }
    }
}

pub fn unbind() {
    unsafe {
        gl::UseProgram(0);
    }
}

const SHADER_INFO_LOG_LEN: usize = 1024;

fn check_compile_status(shader: u32) {
    event!(
        name: "shader.compile.begin",
        Level::INFO,
        "Compiling shader {shader}..."
    );
    let mut log_buf = [0i8; SHADER_INFO_LOG_LEN];
    let mut compile_status = 0;

    unsafe {
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compile_status);
    }

    if compile_status as u8 != gl::TRUE {
        let mut log_len = 0;
        unsafe {
            gl::GetShaderInfoLog(
                shader,
                SHADER_INFO_LOG_LEN as i32,
                &mut log_len,
                log_buf.as_mut_ptr(),
            );
        }
        let log = unsafe { std::ffi::CStr::from_ptr(log_buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        event!(
            name: "shader.compile.fail",
            Level::INFO,
            r#"Failed to compile shader {shader}:
        {log}"#
        );
        panic!(
            r#"OpenGL failed to compile shader ({shader}):
{log}"#,
        )
    }
}

fn check_link_status(program: u32) {
    let mut log_buf = [0i8; SHADER_INFO_LOG_LEN];
    let mut link_status = 0;

    unsafe {
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status);
    }

    if link_status as u8 != gl::TRUE {
        let mut log_len = 0;
        unsafe {
            gl::GetProgramInfoLog(
                program,
                SHADER_INFO_LOG_LEN as i32,
                &mut log_len,
                log_buf.as_mut_ptr(),
            );
        }
        let log = unsafe { std::ffi::CStr::from_ptr(log_buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        panic!(
            r#"OpenGL failed to link shader program:
{log}"#,
        )
    }
}

//! This is all unstable and will be subject to change.
//!
//! A fully compile-time static model is planned.

use std::io::BufRead;

use janus::gl;
use tracing::{Level, event};

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Default, Debug)]
pub struct UniformLocation(i32);

impl std::ops::Deref for UniformLocation {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShaderHandle {
    gl_obj: u32,
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

        Self { gl_obj: program }
    }

    pub fn uniform_location(&self, name: &str) -> UniformLocation {
        // todo: cache uniform locations
        let c_name = std::ffi::CString::new(name).unwrap();
        UniformLocation(unsafe { gl::GetUniformLocation(self.gl_obj, c_name.as_ptr()) })
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

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.gl_obj);
        }
    }

    pub fn unbind() {
        self::unbind();
    }
}

impl Drop for ShaderHandle {
    fn drop(&mut self) {
        if self.gl_obj == 0 {
            return;
        }
        unsafe { gl::DeleteProgram(self.gl_obj) }
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

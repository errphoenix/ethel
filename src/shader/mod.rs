pub mod glsl;
pub mod uniform;

pub use crate::shader_glsl_ssbo;

use std::{hash::Hash, str::FromStr};

use janus::{GlProperty, gl};
use tracing::{Level, event};

use crate::shader::{
    glsl::{GlslAlloc, GlslType, ShadingVersion},
    uniform::GlslUniform,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShaderKind {
    Compute,
    TesselationEval,
    TesselationCtl,
    Vertex,
    Geometry,
    Pixel,
}

impl ShaderKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Compute => "compute",
            Self::TesselationEval => "tess-eval",
            Self::TesselationCtl => "tess-ctl",
            Self::Vertex => "vertex",
            Self::Geometry => "geometry",
            Self::Pixel => "pixel",
        }
    }
}

impl std::fmt::Display for ShaderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl janus::GlProperty for ShaderKind {
    fn property_enum(self) -> u32 {
        match self {
            ShaderKind::Vertex => janus::gl::VERTEX_SHADER,
            ShaderKind::Pixel => janus::gl::FRAGMENT_SHADER,
            ShaderKind::Compute => janus::gl::COMPUTE_SHADER,
            ShaderKind::Geometry => janus::gl::GEOMETRY_SHADER,
            ShaderKind::TesselationEval => janus::gl::TESS_EVALUATION_SHADER,
            ShaderKind::TesselationCtl => janus::gl::TESS_CONTROL_SHADER,
        }
    }
}

pub fn generate_blank() -> ShaderHandle {
    let program = unsafe { janus::gl::CreateProgram() };
    ShaderHandle { prog_obj: program }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShaderUnit {
    pub kind: ShaderKind,
    shader_obj: u32,
}

const SHADER_INFOLOG_LEN: usize = 1024;

static mut SHADER_INFOLOG_BYTES: [i8; SHADER_INFOLOG_LEN] = [0i8; SHADER_INFOLOG_LEN];

pub fn compile_shader_unit(
    source: &[u8],
    shader_kind: ShaderKind,
) -> Result<ShaderUnit, std::borrow::Cow<'_, str>> {
    let shader_obj = unsafe { janus::gl::CreateShader(shader_kind.property_enum()) };

    #[allow(static_mut_refs)]
    {
        let mut compile_status = 0;

        unsafe {
            let c_src = std::ffi::CString::from_raw(source.as_ptr() as *mut i8);

            janus::gl::ShaderSource(shader_obj, 1, &c_src.as_ptr(), std::ptr::null());
            janus::gl::CompileShader(shader_obj);
            janus::gl::GetShaderiv(shader_obj, janus::gl::COMPILE_STATUS, &mut compile_status);
        }

        if compile_status as u8 != janus::gl::TRUE {
            let mut log_string_len = 0;

            unsafe {
                janus::gl::GetShaderInfoLog(
                    shader_obj,
                    SHADER_INFOLOG_LEN as i32,
                    &mut log_string_len,
                    SHADER_INFOLOG_BYTES.as_mut_ptr(),
                );
            }

            let log_contents = unsafe { std::ffi::CStr::from_ptr(SHADER_INFOLOG_BYTES.as_ptr()) }
                .to_string_lossy();

            event!(
                name: "shader.unit.compile",
                Level::ERROR,
                r#"Failed to compile {shader_kind} shader (handle={}) from source:
            {}"#, shader_obj, log_contents
            );
            return Err(log_contents);
        }
    }

    Ok(ShaderUnit {
        kind: shader_kind,
        shader_obj,
    })
}

pub fn attach_shader_units(program: &ShaderHandle, units: &[ShaderUnit]) {
    units
        .iter()
        .for_each(|&ShaderUnit { shader_obj, .. }| unsafe {
            janus::gl::AttachShader(program.prog_obj, shader_obj);
        });
}

pub fn link_shader_program(program: &ShaderHandle) {
    unsafe {
        janus::gl::LinkProgram(program.prog_obj);
        janus::gl::ValidateProgram(program.prog_obj);
    }
}

pub fn delete_shader_units(units: &mut [ShaderUnit]) {
    units.iter_mut().for_each(|ShaderUnit { shader_obj, .. }| {
        unsafe {
            janus::gl::DeleteShader(*shader_obj);
        }
        *shader_obj = 0;
    });
}

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

    pub fn copy_from(&mut self, other: &ShaderComposer) {
        self.header = format!("{}{}", self.header, other.header);
        self.uniforms_section = format!("{}{}", self.uniforms_section, other.uniforms_section);
        self.body = format!("{}{}", self.body, other.body);
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

    pub fn build(self) -> String {
        format!(
            "{}\n{}{}{}{}",
            self.version, self.header, self.uniforms_section, self.body, self.source
        )
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
        Self(format!("void main() {{\n{source}}}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Compose a complete shader p1rogram pass from just one macro invocation.
///
/// The macro presents two sections:
/// * The `common` shader information
/// * The specific information and `main()` source for each shader.
///
/// The `common` and specific shader information must be provided in the given
/// order:
/// 1. Attributes (`attribs`), valid only for specific shader information.
///    See [`crate::shader_glsl_attribs`].
/// 2. Uniforms (`uniform`), with the `shader_name: shader_type => RustType;`
///    syntax.
/// 3. Custom types (`type`), to define custom types to be used in uniforms,
///    SSBO's, etc. See [`crate::shader_glsl_struct`].
/// 4. Shader Storage Buffer Objects (`ssbo`), to define SSBO binding points
///    and types. See [`crate::shader_glsl_ssbo`].
/// 5. Constants (`const`), to define constant variables directly form Rust
///    source doe. See [`crate::shader::Constant`].
/// 6. Utility/library functions (`lib`), to define utility or auxiliary shader
///    functions with a custom syntax. See [`crate::shader::shader_glsl_lib`].
#[macro_export]
macro_rules! shader_glsl {
    (
        struct $name:ident > [$ver:expr] {
            common {
                $(uniform {
                    $(
                        $c_u_gl_name:ident: $c_u_gl_type:ident => $c_u_r_type:ty;
                    )+
                };)?
                $(type {
                    $(
                        $c_type_glsl:expr
                    )+
                };)?
                $(ssbo {
                    $(
                        $c_ssbo_glsl:expr
                    )+
                };)?
                $(const {
                    $(
                        $c_const_a:expr
                    )+
                };)?
                $(lib {
                    $(
                        $c_lib:expr;
                    )+
                };)?
            };

            $(
                unit $kind:expr => [
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
                            $type_glsl:item
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
                ];
            )+
        }
    ) => {
        paste::paste! {
            #[derive(PartialEq, Eq, Hash)]
            pub struct [< Shader $name >] {
                handle: $crate::shader::ShaderHandle,

                $(
                    $(
                        [< location_ $c_u_gl_name _ $c_u_gl_type >]: $crate::shader::UniformLocation,
                    )+
                )?
                $(
                    $(
                        $(
                            [< location_ $u_gl_name _ $u_gl_type >]: $crate::shader::UniformLocation,
                        )+
                    )?
                )+
            }

            impl [< Shader $name >] {
                pub fn bind(&self) {
                    self.handle.bind();
                }

                pub fn unbind(&self) {
                    $crate::shader::unbind();
                }

                pub fn handle(&self) -> &$crate::shader::ShaderHandle {
                    &self.handle
                }

                #[cfg(debug_assertions)]
                pub fn build_sources() -> Vec<String> {
                    let mut sources = Vec::new();

                    let version = $crate::shader::ShadingVersion::core($ver);

                    let common = {
                        let mut composer = $crate::shader::ShaderComposer::new(version);
                        $(
                            $(
                                composer.add_uniform($crate::shader_glsl_uniform!($c_u_gl_name: $c_u_gl_type));
                            )+
                        )?
                        $(
                            $(
                                composer.inject_header(&$c_type_glsl);
                            )+
                        )?
                        $(
                            $(
                                composer.inject_header(&$c_ssbo_glsl);
                            )+
                        )?
                        $(
                            $(
                                composer.add_constant(&$c_const_a);
                            )+
                        )?
                        $(
                            $(
                                composer.inject_body(&$c_lib);
                            )+
                        )?

                        composer
                    };

                    $(
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
                        composer.copy_from(&common);
                        composer.set_source(indoc::indoc! { $src });

                        sources.push(composer.build());
                    )+

                    sources
                }

                $(
                    $(
                        $crate::shader_glsl_build_uniform_interface! {
                            $c_u_gl_name: $c_u_gl_type => $c_u_r_type
                        }
                    )+
                )?
                $(
                    $(
                        $(
                            $crate::shader_glsl_build_uniform_interface! {
                                $u_gl_name: $u_gl_type => $u_r_type
                            }
                        )+
                    )?
                )+

                pub fn new_compiled() -> Self {
                    let mut units = Vec::new();

                    {
                        let version = $crate::shader::ShadingVersion::core($ver);

                        let common = {
                            let mut composer = $crate::shader::ShaderComposer::new(version);
                            $(
                                $(
                                    composer.add_uniform($crate::shader_glsl_uniform!($c_u_gl_name: $c_u_gl_type));
                                )+
                            )?
                            $(
                                $(
                                    composer.inject_header(&$c_type_glsl);
                                )+
                            )?
                            $(
                                $(
                                    composer.inject_header(&$c_ssbo_glsl);
                                )+
                            )?
                            $(
                                $(
                                    composer.add_constant(&$c_const_a);
                                )+
                            )?
                            $(
                                $(
                                    composer.inject_body(&$c_lib);
                                )+
                            )?

                            composer
                        };

                        $(
                            let composer = {
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
                                composer.copy_from(&common);
                                composer.set_source(indoc::indoc! { $src });
                                composer
                            };

                            let full_source = composer.build();
                            let shader_unit = $crate::shader::compile_shader_unit(full_source.as_bytes(), $kind)
                                .expect(concat!("failed to compile ", stringify!($kind), " shader: see logs for details."));

                            units.push(shader_unit);
                        )+
                    }

                    let handle = $crate::shader::generate_blank();
                    $crate::shader::attach_shader_units(&handle, &units);
                    $crate::shader::link_shader_program(&handle);
                    $crate::shader::delete_shader_units(&mut units);

                    $(
                        $(
                            let [< location_ $c_u_gl_name _ $c_u_gl_type >] = handle.find_uniform_location(stringify!($c_u_gl_name));
                        )+
                    )?
                    $(
                        $(
                            $(
                                let [< location_ $u_gl_name _ $u_gl_type >] = handle.find_uniform_location(stringify!($u_gl_name));
                            )+
                        )?
                    )+

                    Self {
                        handle,

                        $(
                            $(
                                [< location_ $c_u_gl_name _ $c_u_gl_type >],
                            )+
                        )?
                        $(
                            $(
                                $(
                                    [< location_ $u_gl_name _ $u_gl_type >],
                                )+
                            )?
                        )+
                    }
                }
            }
        }
    };
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderHandle {
    prog_obj: u32,
}

impl ShaderHandle {
    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.prog_obj);
        }
    }

    pub fn unbind() {
        self::unbind();
    }

    pub fn find_uniform_location(&self, uniform_name: &str) -> UniformLocation {
        let c_string = std::ffi::CString::from_str(uniform_name).unwrap();
        let location = unsafe { janus::gl::GetUniformLocation(self.prog_obj, c_string.as_ptr()) };
        UniformLocation(location)
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

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::*;

    crate::shader_glsl_struct! {
        struct DirectIndex {
            handle: u32 => uint;
            generation: u32 => uint;
        }
    }

    shader_glsl! {
        struct Debug > [460] {
            common {
                uniform {
                    projection: mat4 => glam::Mat4;
                };

                type {
                    DirectIndexGlslStruct::as_definition()
                };

                ssbo {
                    crate::shader_glsl_ssbo! {
                        buf POD_Positions on 1 => {
                            [dyn_array vec4: pod_positions]
                        }
                    }

                    crate::shader_glsl_ssbo! {
                        buf IMap_Entity on 2 => {
                            [dyn_array DirectIndex: imap_entity]
                        }
                    }
                };
            };

            unit ShaderKind::Vertex => [
                const {
                    Constant::new("FIXED_POS", 1.0)
                };

                src() "
                    do cool stuff
                    gl_Position = vec4(FIXED_POS);
                "
            ];

            unit ShaderKind::Pixel => [
                attribs {
                    crate::shader_glsl_attribs! {
                        output outColor: vec4;
                    }
                };

                uniform {
                    view: mat4 => glam::Mat4;
                };

                lib {
                    crate::shader_glsl_lib! {
                        float halve [ num: float ] => "
                            return num * 0.5;
                        "
                    };
                };

                src() "
                    do more cool stuff
                    outColor = vec4(1.0);
                "
            ];
        }
    }

    #[test]
    fn compose_full_shader() {
        let sources = ShaderDebug::build_sources();

        sources
            .iter()
            .enumerate()
            .for_each(|(i, src)| println!("N={i}\n{src}\n\n\n\n\n"));

        const S0: &str = indoc::indoc! { "# version 460 core

            const float FIXED_POS = 1.000;

            struct DirectIndex {
              uint handle;
              uint generation;
            };

            layout(std430, binding = 1) buffer POD_Positions
            {
                vec4 pod_positions[];
            };

            layout(std430, binding = 2) buffer IMap_Entity
            {
                DirectIndex imap_entity[];
            };

            uniform mat4 projection;

            void main() {
            do cool stuff
            gl_Position = vec4(FIXED_POS);
            }" };

        assert_eq!(sources[0].trim_end(), S0);

        const S1: &str = indoc::indoc! { "# version 460 core

            out vec4 outColor;

            struct DirectIndex {
              uint handle;
              uint generation;
            };

            layout(std430, binding = 1) buffer POD_Positions
            {
                vec4 pod_positions[];
            };

            layout(std430, binding = 2) buffer IMap_Entity
            {
                DirectIndex imap_entity[];
            };

            uniform mat4 view;

            uniform mat4 projection;

            float halve(float num) {
            return num * 0.5;
            }

            void main() {
            do more cool stuff
            outColor = vec4(1.0);
            }" };

        assert_eq!(sources[1].trim_end(), S1);
    }
}

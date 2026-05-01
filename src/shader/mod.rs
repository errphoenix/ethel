pub mod glsl;
pub mod uniform;

pub use crate::shader_glsl_ssbo;
use crate::state::data;

use std::{
    hash::Hash,
    ops::Deref,
    str::FromStr,
    sync::atomic::{AtomicU32, Ordering},
};

use janus::{GlProperty, gl};
use tracing::{Level, event};

pub use glsl::{
    Glsl, GlslAlloc, GlslAttribute, GlslLib, GlslStorage, GlslStruct, GlslType, ShadingVersion,
};
pub use uniform::GlslUniform;

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
    source: &str,
    shader_kind: ShaderKind,
) -> Result<ShaderUnit, std::borrow::Cow<'_, str>> {
    let shader_obj = unsafe { janus::gl::CreateShader(shader_kind.property_enum()) };

    #[allow(static_mut_refs)]
    {
        let mut compile_status = 0;

        unsafe {
            let c_src = std::ffi::CString::from_str(source).unwrap();

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

impl WriteValue for data::IndirectIndex {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "IndirectIndex({}, {})", self.index, self.generation)
    }
}

impl WriteValue for data::DirectIndex {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "DirectIndex({}, {})", self.index, self.generation)
    }
}

impl<T: WriteValue, const N: usize> WriteValue for [T; N] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "[")?;
        for (i, v) in self.iter().enumerate() {
            if i != 0 {
                write!(to, ", ")?;
            }
            v.write_value(to)?;
        }
        write!(to, "]")?;

        Ok(())
    }
}

pub trait Inject {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result;
}

pub trait ShaderHeader: Inject {}

pub trait ShaderBody: Inject {}

#[derive(Clone, Debug)]
pub struct Constant<T: Clone + Copy + WriteValue> {
    name: &'static str,
    value: T,
}

impl<T: Clone + Copy + WriteValue> Constant<T> {
    pub const fn new(name: &'static str, value: T) -> Self {
        Self { name, value }
    }

    pub const fn name(&self) -> &str {
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
        self.header = format!("{}{}", other.header, self.header);
        self.uniforms_section = format!("{}{}", other.uniforms_section, self.uniforms_section);
        self.body = format!("{}{}", other.body, self.body);
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

pub trait ShaderProgram: janus::GpuResource {
    fn shader_program(&self) -> u32 {
        self.resource_id()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShaderHandle {
    prog_obj: u32,
}

impl janus::GpuResource for ShaderHandle {
    fn resource_id(&self) -> u32 {
        self.prog_obj
    }
}

impl ShaderProgram for ShaderHandle {}

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

#[derive(Debug)]
pub struct ComputeShaderHandle {
    inner: ShaderHandle,
    workgroups_x: AtomicU32,
    workgroups_y: AtomicU32,
    workgroups_z: AtomicU32,
}

impl Default for ComputeShaderHandle {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl ComputeShaderHandle {
    pub const fn new(handle: ShaderHandle) -> Self {
        Self {
            inner: handle,
            workgroups_x: AtomicU32::new(1),
            workgroups_y: AtomicU32::new(1),
            workgroups_z: AtomicU32::new(1),
        }
    }

    pub fn set_workgroups_size(&self, x: u32, y: u32, z: u32) {
        self.workgroups_x.store(x, Ordering::Relaxed);
        self.workgroups_y.store(y, Ordering::Relaxed);
        self.workgroups_z.store(z, Ordering::Relaxed);
    }

    pub fn workgroups_size(&self) -> (u32, u32, u32) {
        let wg_x = self.workgroups_x.load(Ordering::Relaxed);
        let wg_y = self.workgroups_y.load(Ordering::Relaxed);
        let wg_z = self.workgroups_z.load(Ordering::Relaxed);
        (wg_x, wg_y, wg_z)
    }

    pub fn dispatch_compute(&self) {
        let (x, y, z) = self.workgroups_size();
        unsafe {
            janus::gl::DispatchCompute(x, y, z);
        }
    }
}

impl Deref for ComputeShaderHandle {
    type Target = ShaderHandle;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Compose a complete shader program pass from just one macro invocation.
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
                ];
            )+
        }
    ) => {
        paste::paste! {
            #[derive(Debug, PartialEq, Eq, Hash, Default)]
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
                            let shader_unit = $crate::shader::compile_shader_unit(&full_source, $kind)
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

/// Compose a stand-alone compute shader.
///
/// The macro syntax is just like [`crate::shader_glsl`], with the exception of
/// the `common` and `attribs` sections.
///
/// This macro also features a non-optional `workgroup` section, to define the
/// compute shader's local workgroup size with the `[x, y, z]` syntax.
///
/// The `workgroup` section must be defined before all other sections. The
/// order for all other standard shader sections is the same as defined in
/// [`crate::shader_glsl`].
#[macro_export]
macro_rules! shader_glsl_compute {
    (
        struct $name:ident > [$ver:expr] {
            workgroup [$wg_x:expr, $wg_y:expr, $wg_z:expr];

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
            #[derive(Debug, Default)]
            pub struct [< ComputeShader $name >] {
                handle: $crate::shader::ComputeShaderHandle,

                $(
                    $(
                        [< location_ $u_gl_name _ $u_gl_type >]: $crate::shader::UniformLocation,
                    )+
                )?
            }

            impl [< ComputeShader $name >] {
                pub fn bind(&self) {
                    self.handle.bind();
                }

                pub fn unbind(&self) {
                    $crate::shader::unbind();
                }

                pub fn set_workgroups_size(&self, x: u32, y: u32, z: u32) {
                    self.handle.set_workgroups_size(x, y, z);
                }

                pub fn dispatch(&self) {
                    self.handle.dispatch_compute();
                }

                pub fn handle(&self) -> &$crate::shader::ShaderHandle {
                    &self.handle
                }

                #[cfg(debug_assertions)]
                pub fn build_sources() -> String {
                    let version = $crate::shader::ShadingVersion::core($ver);

                    let mut composer = $crate::shader::ShaderComposer::new(version);
                    {
                        const WORK_GROUP_GLSL: &str = concat!(
                            "layout(local_size_x = ", $wg_x,
                            ", local_size_y = ", $wg_y,
                            ", local_size_z = ", $wg_z, ") in;\n"
                        );
                        composer.inject_header(&$crate::shader::glsl::GlslWorkGroupSize::new(WORK_GROUP_GLSL));
                    }

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

                    composer.build()
                }

                $(
                    $(
                        $crate::shader_glsl_build_uniform_interface! {
                            $u_gl_name: $u_gl_type => $u_r_type
                        }
                    )+
                )?

                pub fn new_compiled() -> Self {
                    let version = $crate::shader::ShadingVersion::core($ver);

                    let mut composer = $crate::shader::ShaderComposer::new(version);

                    {
                        const WORK_GROUP_GLSL: &str = concat!(
                            "layout(local_size_x = ", $wg_x,
                            ", local_size_y = ", $wg_y,
                            ", local_size_z = ", $wg_z, ") in;\n"
                        );
                        composer.inject_header(&$crate::shader::glsl::GlslWorkGroupSize::new(WORK_GROUP_GLSL));
                    }

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

                    let full_source = composer.build();
                    let shader_unit = $crate::shader::compile_shader_unit(&full_source, $crate::shader::ShaderKind::Compute)
                        .expect(concat!("failed to compile Compute shader: see logs for details."));

                    let handle = $crate::shader::ComputeShaderHandle::new($crate::shader::generate_blank());
                    $crate::shader::attach_shader_units(&handle, &[shader_unit]);
                    $crate::shader::link_shader_program(&handle);
                    $crate::shader::delete_shader_units(&mut [shader_unit]);

                    $(
                        $(
                            let [< location_ $u_gl_name _ $u_gl_type >] = handle.find_uniform_location(stringify!($u_gl_name));
                        )+
                    )?

                    Self {
                        handle,

                        $(
                            $(
                                [< location_ $u_gl_name _ $u_gl_type >],
                            )+
                        )?
                    }
                }
            }
        }
    };
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

    macro_rules! ssbo_binding {
        (POD_Positions) => {
            1
        };
        (IMap_Entity) => {
            2
        };
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
                        buf POD_Positions => {
                            [dyn_array vec4: pod_positions]
                        }
                    }

                    crate::shader_glsl_ssbo! {
                        buf IMap_Entity => {
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

            const float FIXED_POS = 1.000;

            uniform mat4 projection;

            void main() {
            do cool stuff
            gl_Position = vec4(FIXED_POS);
            }" };

        assert_eq!(sources[0].trim_end(), S0);

        const S1: &str = indoc::indoc! { "# version 460 core

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

            out vec4 outColor;

            uniform mat4 projection;

            uniform mat4 view;

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

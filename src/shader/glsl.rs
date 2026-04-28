use std::ops::Deref;

use crate::shader::WriteValue;

#[derive(Clone, Copy, Debug)]
pub struct GlslStack<G: Glsl> {
    _marker: std::marker::PhantomData<G>,
}

impl<G: Glsl> GlslStack<G> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GlslHeap<G: GlslAlloc>(pub G);

impl<G: GlslAlloc> GlslHeap<G> {
    pub fn new(value: G) -> Self {
        Self(value)
    }

    pub fn get(&self) -> &G {
        &self.0
    }
}

impl<G: GlslAlloc> Deref for GlslHeap<G> {
    type Target = G;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<G: Glsl> super::Inject for GlslStack<G> {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        let glsl = G::to_glsl();
        to.write_str(glsl)?;
        to.write_char('\n')?;
        Ok(())
    }
}
impl<G: GlslAlloc> super::Inject for GlslHeap<G> {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        let glsl = self.to_glsl_alloc();
        to.write_str(&glsl)?;
        to.write_char('\n')?;
        Ok(())
    }
}

pub trait GlslAlloc {
    fn to_glsl_alloc(&self) -> String;
}

pub trait Glsl {
    fn to_glsl() -> &'static str;
}

pub trait GlslType {
    fn to_glsl_type() -> &'static str;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadingVersion {
    version: u32,
    core: bool,
}

impl ShadingVersion {
    pub const LATEST: Self = Self::core(460);

    pub const fn new(version: u32, core: bool) -> Self {
        Self { version, core }
    }

    pub const fn core(version: u32) -> Self {
        Self {
            version,
            core: true,
        }
    }

    pub const fn is_core(&self) -> bool {
        self.core
    }

    pub const fn version_num(&self) -> u32 {
        self.version
    }
}

impl std::fmt::Display for ShadingVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_glsl_alloc())
    }
}

impl GlslAlloc for ShadingVersion {
    fn to_glsl_alloc(&self) -> String {
        format!(
            "# version {} {}",
            self.version,
            self.core.then(|| "core").unwrap_or_default()
        )
    }
}

impl<T: Clone + Copy + GlslType + super::WriteValue> GlslAlloc for super::Constant<T> {
    fn to_glsl_alloc(&self) -> String {
        let mut f = format!(
            "const {} {} = ",
            T::to_glsl_type(),
            self.name.to_uppercase()
        );
        self.value
            .write_value(&mut f)
            .expect("failed to write value to glsl constant");
        f += ";";
        f
    }
}

macro_rules! copy_type_name_glsl {
    ($gt:ty => $lab:literal) => {
        impl $crate::shader::glsl::Glsl for $gt {
            fn to_glsl() -> &'static str {
                $lab
            }
        }

        impl $crate::shader::glsl::GlslType for $gt {
            fn to_glsl_type() -> &'static str {
                $lab
            }
        }
    };
}

impl WriteValue for f32 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "{:.3}", self)
    }
}

impl WriteValue for u32 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "{}", self)
    }
}

impl WriteValue for i32 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "{}", self)
    }
}

impl WriteValue for bool {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "{}", self)
    }
}

copy_type_name_glsl!(f32 => "float");
copy_type_name_glsl!(i32 => "int");
copy_type_name_glsl!(u32 => "uint");
copy_type_name_glsl!(bool => "boolean");
copy_type_name_glsl!(glam::Vec2 => "vec2");
copy_type_name_glsl!([f32; 2] => "vec2");
copy_type_name_glsl!(glam::Vec3 => "vec3");
copy_type_name_glsl!([f32; 3] => "vec3");
copy_type_name_glsl!(glam::Vec4 => "vec4");
copy_type_name_glsl!([f32; 4] => "vec4");
copy_type_name_glsl!(glam::Mat2 => "mat2");
copy_type_name_glsl!(glam::Mat3 => "mat3");
copy_type_name_glsl!([f32; 9] => "mat3");
copy_type_name_glsl!(glam::Mat4 => "mat4");
copy_type_name_glsl!([f32; 16] => "mat4");

impl super::WriteValue for [f32; 2] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec2({:.3}, {:.3})", self[0], self[1])
    }
}

impl super::WriteValue for [f32; 3] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec3({:.3}, {:.3}, {:.3})", self[0], self[1], self[2])
    }
}

impl super::WriteValue for [f32; 4] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(
            to,
            "vec4({:.3}, {:.3}, {:.3}, {:.3})",
            self[0], self[1], self[2], self[3]
        )
    }
}

impl super::WriteValue for glam::Vec2 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec2({:.3}, {:.3})", self[0], self[1])
    }
}

impl super::WriteValue for glam::Vec3 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec3({:.3}, {:.3}, {:.3})", self[0], self[1], self[2])
    }
}

impl super::WriteValue for glam::Vec4 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(
            to,
            "vec4({:.3}, {:.3}, {:.3}, {:.3})",
            self[0], self[1], self[2], self[3]
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlslAttribute(&'static str);

impl std::fmt::Display for GlslAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslAttribute {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(&self) -> &str {
        self.0
    }
}

#[macro_export]
macro_rules! shader_glsl_attribs {
    (
        input $gl_n:ident: $gl_t:ident;
    ) => {
        GlslAttribute::new(concat!(
            "in",
            " ",
            stringify!($gl_t),
            " ",
            stringify!($gl_n),
            ";"
        ))
    };
    (
        output $gl_n:ident: $gl_t:ident;
    ) => {
        GlslAttribute::new(concat!(
            "out",
            " ",
            stringify!($gl_t),
            " ",
            stringify!($gl_n),
            ";"
        ))
    };
    (
        $(input $i_gl_n:ident: $i_gl_t:ident;)*
        $(output $o_gl_n:ident: $o_gl_t:ident;)*
    ) => {
        GlslAttribute::new(concat!(
            $(
                "in",
                " ",
                stringify!($i_gl_t),
                " ",
                stringify!($i_gl_n),
                ";\n",
            )*
            $(
                "out",
                " ",
                stringify!($o_gl_t),
                " ",
                stringify!($o_gl_n),
                ";\n",
            )*
        ))
    };
}

impl super::Inject for GlslAttribute {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{}\n", self.0)
    }
}

impl super::ShaderHeader for GlslAttribute {}

#[derive(Clone, Debug)]
pub struct GlslStruct(&'static str);

impl std::fmt::Display for GlslStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslStruct {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(&self) -> &str {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct GlslStorage(&'static str);

impl std::fmt::Display for GlslStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslStorage {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(&self) -> &str {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct GlslLib(&'static str);

impl std::fmt::Display for GlslLib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslLib {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(&self) -> &str {
        self.0
    }
}

impl super::Inject for GlslStorage {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{}\n", self.0)
    }
}

impl super::Inject for GlslStruct {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{}\n", self.0)
    }
}

impl super::Inject for GlslLib {
    fn inject_shader(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(to, "{}\n", self.0)
    }
}

impl super::ShaderHeader for GlslStorage {}

impl super::ShaderHeader for GlslStruct {}

impl super::ShaderBody for GlslLib {}

/// Generate a Glsl struct from the given data structure.
///
/// Also creates a Rust struct by the same name and fields, deriving the
/// `Clone`, `Copy`, and `Default` traits.
///
/// Glsl compatibility is given from the automatic implementation of the `Glsl`
/// and `GlslAlloc` traits:
/// * `Glsl` will return a static Glsl struct declaration.
/// * `GlslAlloc` requires a `String` allocation on the heap, and will
#[macro_export]
macro_rules! shader_glsl_struct {
    (
        struct $name:ident {
            $(
                $f_name:ident: $f_typ:ty => $f_lit:ident;
            )+
        }
    ) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug)]
            pub struct [< $name GlslStruct >] {
                $(
                    pub $f_name: $f_typ,
                )+
            }

            impl [< $name GlslStruct >] {
                pub const fn as_definition_str() -> &'static str {
                    concat!(
                        "struct ", stringify!($name), " {\n",
                        $(
                            "  ", stringify!($f_lit), " ", stringify!($f_name), ";\n",
                        )+
                        "};"
                    )
                }

                pub const fn as_definition() -> $crate::shader::glsl::GlslStruct {
                    $crate::shader::glsl::GlslStruct::new(
                        Self::as_definition_str()
                    )
                }
            }

            impl $crate::shader::glsl::Glsl for [< $name GlslStruct >] {
                fn to_glsl() -> &'static str {
                    Self::as_definition_str()
                }
            }

            impl From<[< $name GlslStruct >]> for $crate::shader::glsl::GlslStruct {
                fn from(_: [< $name GlslStruct >]) -> Self {
                    Self::new(<[< $name GlslStruct >] as $crate::shader::glsl::Glsl>::to_glsl())
                }
            }

            impl $crate::shader::glsl::GlslType for [< $name GlslStruct >] {
                fn to_glsl_type() -> &'static str {
                    stringify!($name)
                }
            }

            impl $crate::shader::glsl::GlslAlloc for [< $name GlslStruct >] {
                fn to_glsl_alloc(&self) -> String {
                    let mut s = format!("{}(", stringify!($name));

                    $(
                        $crate::shader::WriteValue::write_value(&self.$f_name, &mut s);
                        s += ", ";
                    )+

                    format!("{});", s.trim_end_matches(", "))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! shader_glsl_ssbo {
    (
        buf $ssbo:ident on $index:expr => {
            $(
                $t:ident : $n:ident;
            )*
            $(
                [dyn_array $dat:ident: $dan:ident $(=> each $len:expr)?]
            )?
        }
    ) => {
        $crate::shader::glsl::GlslStorage::new(
            concat!("layout(std430, binding = ", stringify!($index), ") buffer ",
                stringify!($ssbo), "\n {\n",
                $("    ", stringify!($t), " ", stringify!($n), ";\n",)*
                $("    ", stringify!($dat), " ", stringify!($dan), "[]",
                    $("[", $len, "]",)?
                    ";\n",)?
                "};\n")
        )
    };
}

#[macro_export]
macro_rules! shader_glsl_lib {
    (
        $return:ident $fun_name:ident [
            $($par_n_0:ident: $par_t_0:ident $(, $par_n_n:ident: $par_t_n:ident)*)?
        ] =>
            $lib_src:literal

    ) => {
        $crate::shader::glsl::GlslLib::new(
            concat!(
                stringify!($return), " ", stringify!($fun_name), "(",
                $(stringify!($par_t_0), " ", stringify!($par_n_0), $(", ", stringify!($par_t_n), " ", stringify!($par_n_n),)*)?
                ") {\n", indoc::indoc! { $lib_src }, "\n}\n"
            )
        )
    };
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn shader_compose_glsl_lib() {
        const TEST: &str =
            "float mulBySeven(float num) {\nfloat result = num * 7.0;\nreturn result;\n\n}\n";

        let generated = shader_glsl_lib! {
            float mulBySeven [ num: float ] => "
                float result = num * 7.0;
                return result;
            "
        };

        assert_eq!(TEST, generated.as_str());
    }

    #[test]
    fn shader_compose_glsl_version() {
        const TEST: &str = "# version 330 core";

        let version = ShadingVersion::core(330);
        let str = version.to_glsl_alloc();

        assert_eq!(TEST, &str);
    }

    #[test]
    fn shader_compose_glsl_const() {
        const TEST: &str = "const float AMBIENT_LIGHT = 0.100;";

        let constant = Constant::new("ambient_light", 0.1);
        let str = constant.to_glsl_alloc();

        assert_eq!(TEST, &str);
    }

    #[test]
    fn shader_compose_glsl_ssbo() {
        const TEST: &str =
            "layout(std430, binding = 2) buffer POD_BindPose\n {\n    vec4 pod_bind_pose[];\n};\n";

        let generated = shader_glsl_ssbo! {
            buf POD_BindPose on 2 => {
                [dyn_array vec4: pod_bind_pose]
            }
        };

        assert_eq!(TEST, generated.as_str());

        const TEST1: &str =
            "layout(std430, binding = 3) buffer POD_Weights\n {\n    float pod_weights[][2];\n};\n";

        let generated = shader_glsl_ssbo! {
            buf POD_Weights on 3 => {
                [dyn_array float: pod_weights => each 2]
            }
        };

        assert_eq!(TEST1, generated.as_str());
    }

    #[test]
    fn shader_compose_glsl_attribs() {
        const TEST: &str =
            "in vec4 color;\nin vec3 test;\nout vec3 worldPos;\nout vec2 texCoords;\n";

        let generated = shader_glsl_attribs! {
            input color: vec4;
            input test: vec3;
            output worldPos: vec3;
            output texCoords: vec2;
        };

        assert_eq!(TEST, generated.as_str());
    }
}

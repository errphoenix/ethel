use std::ops::Deref;

use crate::shader::header::ShaderHeader;

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

macro_rules! write_value_display {
    ($t:ty) => {
        impl $crate::shader::WriteValue for $t {
            fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
                write!(to, "{self}")
            }
        }
    };
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

write_value_display!(f32);
write_value_display!(i32);
write_value_display!(u32);
write_value_display!(bool);

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

#[derive(Clone, Debug)]
pub struct GlslStruct(String);

impl std::fmt::Display for GlslStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslStruct {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct GlslStorage(String);

impl std::fmt::Display for GlslStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GlslStorage {
    pub fn as_str(&self) -> &str {
        &self.0
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

impl ShaderHeader for GlslStorage {}

impl ShaderHeader for GlslStruct {}

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
macro_rules! glsl_struct {
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

            impl $crate::shader::glsl::Glsl for [< $name GlslStruct >] {
                fn to_glsl() -> &'static str {
                    concat!(
                        "struct ", stringify!($name), " {\n",
                        $(
                            "  ", stringify!($f_lit), " ", stringify!($f_name), ";\n",
                        )+
                        "};"
                    )
                }
            }

            impl From<[< $name GlslStruct >]> for $crate::shader::glsl::GlslStruct {
                fn from(_: [< $name GlslStruct >]) -> Self {
                    Self(<[< $name GlslStruct >] as $crate::shader::glsl::Glsl>::to_glsl().to_string())
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

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn shader_compose_glsl_version() {
        const TEST: &str = "# version 330 core";

        let version = ShadingVersion::core(330);
        let str = version.to_glsl_alloc();

        assert_eq!(TEST, &str);
    }

    #[test]
    fn shader_compose_glsl_const() {
        const TEST: &str = "const float AMBIENT_LIGHT = 0.1;";

        let constant = Constant::new("ambient_light".to_string(), 0.1);
        let str = constant.to_glsl_alloc();

        assert_eq!(TEST, &str);
    }
}

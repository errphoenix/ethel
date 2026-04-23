pub trait GlslAlloc {
    fn to_glsl_alloc(&self) -> String;
}

pub trait Glsl {
    fn to_glsl() -> &'static str;
}

impl<T: Glsl> super::Inject for T {
    fn inject_glsl(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        let glsl = T::to_glsl();
        to.write_str(glsl)?;
        to.write_char('\n')?;
        Ok(())
    }
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

impl<T: Clone + Copy + Glsl + super::WriteValue> GlslAlloc for super::Constant<T> {
    fn to_glsl_alloc(&self) -> String {
        let mut f = format!("const {} {} = ", T::to_glsl(), self.name.to_uppercase());
        self.value
            .write_value(&mut f)
            .expect("failed to write value to glsl constant");
        f += ";";
        f
    }
}

const GLSL_TYPE_FLOAT: &'static str = "float";
const GLSL_TYPE_INT: &'static str = "int";
const GLSL_TYPE_UINT: &'static str = "uint";
const GLSL_TYPE_BOOL: &'static str = "boolean";
const GLSL_TYPE_VEC2: &'static str = "vec2";
const GLSL_TYPE_VEC3: &'static str = "vec3";
const GLSL_TYPE_VEC4: &'static str = "vec4";
const GLSL_TYPE_MAT3: &'static str = "mat3";
const GLSL_TYPE_MAT4: &'static str = "mat4";

impl Glsl for f32 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_FLOAT
    }
}

macro_rules! write_value_display {
    ( $( $t:ty )+ ) => {
        $(
            impl $crate::shader::WriteValue for $t {
                fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
                    write!(to, "{self}")
                }
            }
        )+
    };
}

write_value_display!(f32 i32 u32 bool);

impl super::WriteValue for [f32; 2] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec2({}, {})", self[0], self[1])
    }
}

impl super::WriteValue for [f32; 3] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec3({}, {}, {})", self[0], self[1], self[2])
    }
}

impl super::WriteValue for [f32; 4] {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(
            to,
            "vec4({}, {}, {}, {})",
            self[0], self[1], self[2], self[3]
        )
    }
}

impl super::WriteValue for glam::Vec2 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec2({}, {})", self[0], self[1])
    }
}

impl super::WriteValue for glam::Vec3 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "vec3({}, {}, {})", self[0], self[1], self[2])
    }
}

impl super::WriteValue for glam::Vec4 {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(
            to,
            "vec4({}, {}, {}, {})",
            self[0], self[1], self[2], self[3]
        )
    }
}

impl Glsl for i32 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_INT
    }
}

impl Glsl for u32 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_UINT
    }
}

impl Glsl for bool {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_BOOL
    }
}

impl Glsl for [f32; 3] {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC3
    }
}

impl Glsl for [f32; 4] {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC4
    }
}

impl Glsl for [f32; 2] {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC2
    }
}

impl Glsl for glam::Vec3 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC3
    }
}

impl Glsl for glam::Vec2 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC2
    }
}

impl Glsl for glam::Vec4 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC4
    }
}

impl Glsl for glam::Quat {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_VEC4
    }
}

impl Glsl for glam::Mat3 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_MAT3
    }
}

impl Glsl for glam::Mat4 {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_MAT4
    }
}

impl Glsl for [f32; 9] {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_MAT3
    }
}

impl Glsl for [f32; 16] {
    fn to_glsl() -> &'static str {
        GLSL_TYPE_MAT4
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn shader_compose_glsl_version() {
        let TEST: &str = "# version 330 core";

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

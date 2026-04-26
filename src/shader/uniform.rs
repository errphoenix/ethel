use crate::shader::{ShaderHandle, UniformLocation, glsl::GlslAlloc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UniformKind {
    Matrix4,
    Matrix3,
    Matrix2,
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    Ivec2,
    Ivec3,
    Ivec4,
    Uint,
    Uivec2,
    Uivec3,
    Uivec4,
    Boolean,
}

impl super::glsl::GlslAlloc for UniformKind {
    fn to_glsl_alloc(&self) -> String {
        format!(
            "{}",
            match self {
                UniformKind::Matrix4 => "mat4",
                UniformKind::Matrix3 => "mat3",
                UniformKind::Matrix2 => "mat2",
                UniformKind::Float => "float",
                UniformKind::Vec2 => "vec2",
                UniformKind::Vec3 => "vec3",
                UniformKind::Vec4 => "vec4",
                UniformKind::Int => "int",
                UniformKind::Ivec2 => "ivec2",
                UniformKind::Ivec3 => "ivec3",
                UniformKind::Ivec4 => "ivec4",
                UniformKind::Uint => "uint",
                UniformKind::Uivec2 => "uivec2",
                UniformKind::Uivec3 => "uivec3",
                UniformKind::Uivec4 => "uivec4",
                UniformKind::Boolean => "boolean",
            }
        )
    }
}

impl std::fmt::Display for UniformKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_glsl_alloc())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShaderUniform {
    pub name: &'static str,
    pub kind: UniformKind,
}

impl ShaderUniform {
    pub fn new(name: &'static str, kind: UniformKind) -> Self {
        Self { name, kind }
    }
}

impl super::glsl::GlslAlloc for ShaderUniform {
    fn to_glsl_alloc(&self) -> String {
        format!("uniform {} {};", self.kind, self.name)
    }
}

impl std::fmt::Display for ShaderUniform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_glsl_alloc())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ShaderUniformCache(rustc_hash::FxHashMap<&'static str, UniformLocation>);

impl std::fmt::Display for ShaderUniformCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[")?;
        for (name, loc) in self.0.iter() {
            writeln!(f, "    {name} : {loc}")?;
        }
        writeln!(f, "]")
    }
}

impl ShaderUniformCache {
    pub fn new(uniforms: &[ShaderUniform], shader: ShaderHandle) -> Self {
        Self(
            uniforms
                .iter()
                .map(|ShaderUniform { name, .. }| {
                    let c_name = std::ffi::CString::new(*name).unwrap();
                    let location = UniformLocation(unsafe {
                        janus::gl::GetUniformLocation(shader.prog_obj, c_name.as_ptr())
                    });
                    (*name, location)
                })
                .collect::<rustc_hash::FxHashMap<_, _>>(),
        )
    }

    pub fn get(&self, name: &'static str) -> Option<UniformLocation> {
        self.0.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_compose_glsl_uniform() {
        const TEST: &str = "uniform mat4 projection;";
        let uniform = ShaderUniform::new("projection", UniformKind::Matrix4).to_glsl_alloc();
        assert_eq!(TEST, &uniform);
    }
}

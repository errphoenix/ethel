pub trait ShaderHeader: super::Inject {}

#[macro_export]
macro_rules! ssbo_glsl {
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
        concat!("layout(std430, binding = ", $index, ") buffer ",
            stringify!($ssbo), "\n {\n",
            $("    ", stringify!($t), $n, ";\n",)*
            $("    ", stringify!($dat), " ", stringify!($dan), "[]",
                $("[", $len, "]",)?
                ";\n",)?
            "};\n"
        )
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn compose_ssbo_glsl() {
        const TEST: &str =
            "layout(std430, binding = 2) buffer POD_BindPose\n {\n    vec4 pod_bind_pose[];\n};\n";

        let generated = ssbo_glsl! {
            buf POD_BindPose on 2 => {
                [dyn_array vec4: pod_bind_pose]
            }
        };

        assert_eq!(TEST, generated);

        const TEST1: &str =
            "layout(std430, binding = 3) buffer POD_Weights\n {\n    float pod_weights[][2];\n};\n";

        let generated = ssbo_glsl! {
            buf POD_Weights on 3 => {
                [dyn_array float: pod_weights => each 2]
            }
        };

        assert_eq!(TEST1, generated);
    }
}

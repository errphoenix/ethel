use std::fmt::Write;

pub trait VarShader: Default + PartialEq + Eq + Clone + Copy + std::fmt::Debug {}

pub trait VariantCount {
    const COUNT: usize;
}

#[macro_export]
macro_rules! impl_variant_count {
    ($name:ident { $($variant:ident),* $(,)? }) => {
        impl $crate::shader::source::VariantCount for $name {
            const COUNT: usize = $crate::impl_variant_count!(@count $($variant)*);
        }
    };

    (@count) => { 0 };
    (@count $head:ident $($tail:ident)*) => {
        1 + $crate::impl_variant_count!(@count $($tail)*)
    };
}

#[derive(Clone, Debug)]
pub enum ShaderSourceNode<const VAR_COUNT: usize, T: VarShader> {
    Literal(&'static str),
    Variable(Vec<(T, Box<Self>)>),
    Subtree(Vec<Self>),
}

#[derive(Clone, Debug, Default)]
pub struct ShaderSourceTree<const VAR_COUNT: usize, T: VarShader> {
    nodes: Vec<ShaderSourceNode<VAR_COUNT, T>>,
}

#[derive(Clone, Debug, Default)]
pub struct ShaderSourceBuilder<const VAR_COUNT: usize, T: VarShader> {
    tree: ShaderSourceTree<VAR_COUNT, T>,
}
impl<const VAR_COUNT: usize, T: VarShader> ShaderSourceBuilder<VAR_COUNT, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, node: ShaderSourceNode<VAR_COUNT, T>) {
        self.tree.nodes.push(node);
    }

    fn build_node(variant: T, node: &ShaderSourceNode<VAR_COUNT, T>, out: &mut String) {
        match node {
            ShaderSourceNode::Literal(literal) => {
                let _ = writeln!(out, "{literal}");
            }
            ShaderSourceNode::Variable(options) => {
                if let Some((variant, contents)) = options.iter().find(|(v, _)| variant.eq(v)) {
                    Self::build_node(*variant, contents, out);
                } else {
                    let (_, default) = options.iter().find(|(v, _)| variant.eq(v)).unwrap();
                    Self::build_node(T::default(), default, out);
                }
            }
            ShaderSourceNode::Subtree(shader_source_nodes) => {
                Self::build_nodes(variant, shader_source_nodes, out)
            }
        }
    }

    fn build_nodes(variant: T, nodes: &[ShaderSourceNode<VAR_COUNT, T>], out: &mut String) {
        for node in nodes {
            Self::build_node(variant, node, out);
        }
    }

    pub fn build(&self, variant: T) -> String {
        let mut string = String::new();
        Self::build_nodes(variant, &self.tree.nodes, &mut string);
        format!("void main() {{\n{string}}}")
    }
}

#[macro_export]
macro_rules! shader_source_internal {
    // end of tt
    (@parse $builder:ident, $variants:ident,) => {};
    // direct match literal
    (@parse $builder:ident, $variants:ident, $lit:literal; $($rest:tt)*) => {
        $builder.append(
            $crate::shader::source::ShaderSourceNode::Literal(indoc::indoc!($lit))
        );
        $crate::shader_source_internal!(@parse $builder, $variants, $($rest)*);
    };
    // direct match branch
    (@parse $builder:ident, $variants:ident, match { $($arms:tt)* } $($rest:tt)*) => {
        {
            let mut arms = Vec::new();
            $crate::shader_source_internal!(@parse_arms arms, $variants, $($arms)*);
            $builder.append($crate::shader::source::ShaderSourceNode::Variable(arms));
        }
        $crate::shader_source_internal!(@parse $builder, $variants, $($rest)*);
    };

    // arm: end of tt
    (@parse_arms $arms:ident, $variants:ident,) => {};
    // arm: match literal
    (@parse_arms $arms:ident, $variants:ident,
        $($v:ident)|+ => $lit:literal;
        $($rest:tt)*
    ) => {
        {
            let node = $crate::shader::source::ShaderSourceNode::Literal(indoc::indoc!($lit));
            $(
                $arms.push(($variants::$v, Box::new(node.clone())));
            )*
        }
        $crate::shader_source_internal!(@parse_arms $arms, $variants, $($rest)*);
    };
    // arm: match branch
    (@parse_arms $arms:ident, $variants:ident,
        $($v:ident)|+ => { $($body:tt)* };
        $($rest:tt)*
    ) => {
        {
            let mut subtree = Vec::new();
            $crate::shader_source_internal!(@parse_body subtree, $variants, $($body)*);
            let node = $crate::shader::source::ShaderSourceNode::Subtree(subtree);

            $(
                $arms.push(($variants::$v, Box::new(node.clone())));
            )*
        }
        $crate::shader_source_internal!(@parse_arms $arms, $variants, $($rest)*);
    };
    // arm: fallback match literal
    (@parse_arms $arms:ident, $variants:ident,
        _ => $lit:literal;
        $($rest:tt)*
    ) => {
        $arms.push((
            std::default::Default::default(),
            Box::new($crate::shader::source::ShaderSourceNode::Literal(indoc::indoc!($lit)))
        ));
        $crate::shader_source_internal!(@parse_arms $arms, $variants, $($rest)*);
    };
    // arm: fallback match branch
    (@parse_arms $arms:ident, $variants:ident,
        _ => { $($body:tt)* };
        $($rest:tt)*
    ) => {
        {
            let mut subtree = Vec::new();
            $crate::shader_source_internal!(@parse_body subtree, $variants, $($body)*);
            $arms.push((
                std::default::Default::default(),
                Box::new($crate::shader::source::ShaderSourceNode::Subtree(subtree))
            ));
        }
        $crate::shader_source_internal!(@parse_arms $arms, $variants, $($rest)*);
    };

    // body: end of tt
    (@parse_body $subtree:ident, $variants:ident,) => {};
    // body: match literal
    (@parse_body $subtree:ident, $variants:ident, $lit:literal; $($rest:tt)*) => {
        $subtree.push(
            $crate::shader::source::ShaderSourceNode::Literal(indoc::indoc!($lit))
        );
        $crate::shader_source_internal!(@parse_body $subtree, $variants, $($rest)*);
    };
    // body: match branch
    (@parse_body $subtree:ident, $variants:ident, match { $($arms:tt)* } $($rest:tt)*) => {
        {
            let mut arms = Vec::new();
            $crate::shader_source_internal!(@parse_arms arms, $variants, $($arms)*);
            $subtree.push($crate::shader::source::ShaderSourceNode::Variable(arms));
        }
        $crate::shader_source_internal!(@parse_body $subtree, $variants, $($rest)*);
    };
}

#[macro_export]
macro_rules! shader_source {
    ($variants:ident, $($tokens:tt)*) => {{
        let mut builder = $crate::shader::source::ShaderSourceBuilder::<
            { <$variants as $crate::shader::source::VariantCount>::COUNT },
            $variants
        >::new();
        $crate::shader_source_internal!(@parse builder, $variants, $($tokens)*);
        builder
    }};
}

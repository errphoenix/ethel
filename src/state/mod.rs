use crate::{mesh, state::column::Column};

pub mod column;
pub mod triple_buffer;

// in case a switch to higher precision with f64 is required
pub(crate) type Real = f32;

// X, Y, Z
type Position = [Real; 3];
// Quaternion
type Rotation = [Real; 4];

#[derive(Debug)]
struct Renderable {
    mesh: u32,
    position: u32,
    rotation: u32,
}

#[derive(Debug, Default)]
pub struct State {
    meshes: Column<mesh::Id>,
    positions: Column<Position>,

    renderables: Vec<Renderable>,
    // consider arc-swap or rwlock for render input data
}

impl janus::context::Update for State {
    fn update(&mut self, delta: janus::context::DeltaTime) {
        todo!()
    }
}

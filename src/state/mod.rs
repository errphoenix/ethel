use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{mesh, state::column::Column};

pub mod column;
pub mod cross;

#[derive(Clone, Copy, PartialEq, PartialOrd, Default, Debug)]
pub struct Real(FloatType);

type FloatType = f32;

impl Display for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Real {
    type Target = FloatType;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Real {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Real {
    pub fn new(float: FloatType) -> Self {
        Self(float)
    }

    pub fn as_f32(&self) -> f32 {
        self.0 as f32
    }

    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }
}

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
}

impl janus::context::Update for State {
    fn update(&mut self, delta: janus::context::DeltaTime) {}

    fn step_duration(&self) -> std::time::Duration {
        //todo
        Duration::from_millis(6)
    }

    fn set_step_duration(&mut self, _step: std::time::Duration) {
        //todo
    }
}

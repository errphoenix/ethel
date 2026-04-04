use std::collections::hash_map::{Keys, Values};

use rustc_hash::FxHashMap as HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Cell {
    pub const X: Cell = Cell::new(1, 0, 0);
    pub const Y: Cell = Cell::new(0, 1, 0);
    pub const Z: Cell = Cell::new(0, 0, 1);
    pub const XY: Cell = Cell::new(1, 1, 0);
    pub const YZ: Cell = Cell::new(0, 1, 1);
    pub const ZX: Cell = Cell::new(1, 0, 1);
    pub const NEG_XY: Cell = Cell::new(-1, 1, 0);
    pub const NEG_YZ: Cell = Cell::new(0, -1, 1);
    pub const NEG_ZX: Cell = Cell::new(1, 0, -1);

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }
}

impl std::ops::Neg for Cell {
    type Output = Cell;

    fn neg(self) -> Self::Output {
        Self::Output {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl std::ops::Add for Cell {
    type Output = Cell;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub for Cell {
    type Output = Cell;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Mul<i32> for Cell {
    type Output = Cell;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::Output {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

/// The size of each cell of a spatial hash structure.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct SpatialResolution(f32);

impl Default for SpatialResolution {
    fn default() -> Self {
        Self(Self::DEFAULT_RESOLUTION)
    }
}

impl SpatialResolution {
    pub const DEFAULT_RESOLUTION: f32 = 1.0;

    pub fn new(resolution: f32) -> Self {
        debug_assert!(
            resolution > 0.0,
            "spatial resolution must be greater than 0"
        );
        Self(resolution)
    }

    pub fn get(&self) -> f32 {
        self.0
    }

    #[inline]
    pub fn encode_point(&self, point: glam::Vec3) -> Cell {
        let cell_size = self.0;
        Cell {
            x: ((point.x + cell_size * 0.5).floor() / cell_size) as i32,
            y: ((point.y + cell_size * 0.5).floor() / cell_size) as i32,
            z: ((point.z + cell_size * 0.5).floor() / cell_size) as i32,
        }
    }

    #[inline]
    pub fn approx_point(&self, cell: Cell) -> glam::Vec3 {
        glam::vec3(
            cell.x as f32 / self.0,
            cell.y as f32 / self.0,
            cell.z as f32 / self.0,
        )
    }
}

#[derive(Debug, Clone)]
pub struct FxSpatialHash<T: Clone + Copy> {
    map: HashMap<Cell, T>,

    /// The amount of cells in a 'unit' of space for each axis
    pub resolution: SpatialResolution,
}

impl<T: Default + Clone + Copy> Default for FxSpatialHash<T> {
    fn default() -> Self {
        Self {
            resolution: Default::default(),
            map: Default::default(),
        }
    }
}

impl<T: Clone + Copy> FxSpatialHash<T> {
    pub fn new(resolution: SpatialResolution) -> Self {
        Self {
            resolution,
            map: HashMap::default(),
        }
    }

    pub fn with_capacity(resolution: SpatialResolution, capacity: usize) -> Self {
        Self {
            resolution,
            map: HashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    pub fn cells(&self) -> Keys<'_, Cell, T> {
        self.map.keys()
    }

    pub fn elements(&self) -> Values<'_, Cell, T> {
        self.map.values()
    }

    /// Add an `element` to the spatial hash to a specific `cell`.
    ///
    /// # Returns
    /// The previous element present in `cell`, if any.
    pub fn put(&mut self, cell: Cell, element: T) -> Option<T> {
        self.map.insert(cell, element)
    }

    /// Removes the element placed in `cell`.
    ///
    /// # Returns
    /// The removed elemenet in `cell`, if any.
    pub fn remove(&mut self, cell: Cell) -> Option<T> {
        self.map.remove(&cell)
    }

    /// Get a reference to the element placed in `cell` if existing.
    pub fn get(&self, cell: Cell) -> Option<&T> {
        self.map.get(&cell)
    }

    /// Get an exlusive reference to the element placed in `cell` if existing.
    pub fn get_mut(&mut self, cell: Cell) -> Option<&mut T> {
        self.map.get_mut(&cell)
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn resolution(&self) -> SpatialResolution {
        self.resolution
    }

    #[inline]
    pub fn cell_at(&self, point: glam::Vec3) -> Cell {
        self.resolution.encode_point(point)
    }

    #[inline]
    pub fn approx_point_at(&self, cell: Cell) -> glam::Vec3 {
        self.resolution.approx_point(cell)
    }

    pub fn dump_soa(&mut self, positions: &[glam::Vec3], elements: &[T]) {
        let resolution = self.resolution;
        positions
            .iter()
            .map(|&point| resolution.encode_point(point))
            .zip(elements)
            .for_each(|(cell, &element)| {
                self.put(cell, element);
            });
    }

    pub fn dump_aos(&mut self, data: &[(glam::Vec3, T)]) {
        data.iter().for_each(|&(point, element)| {
            let cell = self.cell_at(point);
            self.put(cell, element);
        });
    }

    fn cell_query_check(
        &self,
        count: &mut u32,
        src_cell: Cell,
        offset_cell: Cell,
        out: &mut Vec<Cell>,
        ignore_self: bool,
    ) -> bool {
        let o_cell = src_cell + offset_cell;

        if self.map.get(&o_cell).is_some() && (!ignore_self || o_cell != src_cell) {
            out.push(o_cell);
            *count = count.saturating_sub(1);
        }
        *count < 1
    }

    /// Get a specific amount `count` of populated cells nearest to `cell`
    /// within `max_range`.
    ///
    /// The found cells will be written to `out` starting from index 0 to
    /// index `count`.
    ///
    /// If `ignore_self` is `true`, the given starting `cell` will be ignored.
    ///
    /// # Returns
    /// * [`Ok`] if all `count` cells were found and written to `out`.
    /// * Otherwise, [`Err`] containing the remaining amount of cells that
    ///   could not be found.
    pub fn nearest_cells(
        &self,
        cell: Cell,
        count: u32,
        max_range: u32,
        out: &mut Vec<Cell>,
        ignore_self: bool,
    ) -> Result<(), u32> {
        let mut rem = count;
        let mut end = false;

        for i in 1..=max_range as i32 {
            // x axis
            for y in -i..=i {
                for z in -i..=i {
                    let offset = Cell::new(i as i32, y, z);
                    let neg_offset = Cell::new(-i as i32, y, z);
                    self.cell_query_check(&mut rem, cell, offset, out, ignore_self);
                    self.cell_query_check(&mut rem, cell, neg_offset, out, ignore_self);
                }
            }

            // y axis
            // skip first and last X cells to avoid duplicates
            for x in (-i + 1)..i {
                for z in -i..=i {
                    let offset = Cell::new(x, i as i32, z);
                    let neg_offset = Cell::new(x, -i as i32, z);
                    self.cell_query_check(&mut rem, cell, offset, out, ignore_self);
                    self.cell_query_check(&mut rem, cell, neg_offset, out, ignore_self);
                }
            }

            // z axis
            // skip first and last XY cells to avoid duplicates
            for x in (-i + 1)..i {
                for y in (-i + 1)..i {
                    let offset = Cell::new(x, y, i as i32);
                    let neg_offset = Cell::new(x, y, -i as i32);
                    self.cell_query_check(&mut rem, cell, offset, out, ignore_self);
                    end = self.cell_query_check(&mut rem, cell, neg_offset, out, ignore_self);
                }
            }
            if end {
                let point = glam::vec3(cell.x as f32, cell.y as f32, cell.z as f32);
                out.sort_by(|&a, &b| {
                    let a = glam::vec3(a.x as f32, a.y as f32, a.z as f32);
                    let b = glam::vec3(b.x as f32, b.y as f32, b.z as f32);
                    let dst_a = point.distance_squared(a);
                    let dst_b = point.distance_squared(b);
                    dst_a
                        .partial_cmp(&dst_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                return Ok(());
            }
        }

        Err(rem)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[derive(Clone, Debug)]
pub struct FxLsSpatialHash<T: Clone + Copy> {
    map: HashMap<Cell, Vec<T>>,

    pub resolution: SpatialResolution,
}

impl<T: Default + Clone + Copy> Default for FxLsSpatialHash<T> {
    fn default() -> Self {
        Self {
            resolution: Default::default(),
            map: Default::default(),
        }
    }
}

impl<T: Clone + Copy> FxLsSpatialHash<T> {
    pub fn new(resolution: SpatialResolution) -> Self {
        Self {
            resolution,
            map: HashMap::default(),
        }
    }

    pub fn with_capacity(resolution: SpatialResolution, capacity: usize) -> Self {
        Self {
            resolution,
            map: HashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    pub fn cells(&self) -> Keys<'_, Cell, Vec<T>> {
        self.map.keys()
    }

    pub fn elements(&self) -> Values<'_, Cell, Vec<T>> {
        self.map.values()
    }

    /// Add an `element` to the spatial hash to a specific `cell`.
    pub fn put(&mut self, cell: Cell, element: T) {
        let vec = self.map.entry(cell).or_insert_with(|| Vec::new());
        vec.push(element);
    }

    /// Removes the element placed in `cell`.
    ///
    /// # Returns
    /// The removed elemenet in `cell`, if any.
    pub fn clear_bucket(&mut self, cell: Cell) -> Option<Vec<T>> {
        self.map.remove(&cell)
    }

    /// Get a reference to the element placed in `cell` if existing.
    pub fn get(&self, cell: Cell) -> Option<&Vec<T>> {
        self.map.get(&cell)
    }

    /// Get an exlusive reference to the element placed in `cell` if existing.
    pub fn get_mut(&mut self, cell: Cell) -> Option<&mut Vec<T>> {
        self.map.get_mut(&cell)
    }

    /// Clears the contents of all buckets, but keeps their allocations.
    ///
    /// Useful when updating the spatial hash every frame.
    pub fn clear(&mut self) {
        self.map.values_mut().for_each(Vec::clear);
    }

    /// Completely trashes all data, deallocating all buckets.
    pub fn empty(&mut self) {
        self.map.clear();
    }

    pub fn resolution(&self) -> SpatialResolution {
        self.resolution
    }

    #[inline]
    pub fn cell_at(&self, point: glam::Vec3) -> Cell {
        self.resolution.encode_point(point)
    }

    #[inline]
    pub fn approx_point_at(&self, cell: Cell) -> glam::Vec3 {
        self.resolution.approx_point(cell)
    }

    pub fn dump_soa(&mut self, positions: &[glam::Vec3], elements: &[T]) {
        let resolution = self.resolution;
        positions
            .iter()
            .map(|&point| resolution.encode_point(point))
            .zip(elements)
            .for_each(|(cell, &element)| {
                self.put(cell, element);
            });
    }

    pub fn dump_aos(&mut self, data: &[(glam::Vec3, T)]) {
        data.iter().for_each(|&(point, element)| {
            let cell = self.cell_at(point);
            self.put(cell, element);
        });
    }
}

use rustc_hash::FxHashMap as HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpatialResolution(u32);

impl Default for SpatialResolution {
    fn default() -> Self {
        Self(Self::DEFAULT_RESOLUTION)
    }
}

impl SpatialResolution {
    pub const DEFAULT_RESOLUTION: u32 = 1;

    pub fn new(resolution: u32) -> Self {
        debug_assert!(resolution > 0, "spatial resolution must be atleast 1");
        Self(resolution)
    }

    pub fn get(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn encode_point(&self, point: glam::Vec3) -> Cell {
        let i_point = point.as_ivec3();
        let base_x = i_point.x * self.0 as i32;
        let base_y = i_point.y * self.0 as i32;
        let base_z = i_point.z * self.0 as i32;

        let rem_x = point.x.fract() * self.0 as f32;
        let rem_y = point.y.fract() * self.0 as f32;
        let rem_z = point.z.fract() * self.0 as f32;

        Cell {
            x: base_x + rem_x as i32,
            y: base_y + rem_y as i32,
            z: base_z + rem_z as i32,
        }
    }
}

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
    pub fn remove(&mut self, cell: &Cell) -> Option<T> {
        self.map.remove(cell)
    }

    /// Get a reference to the element placed in `cell` if existing.
    pub fn get(&self, cell: &Cell) -> Option<&T> {
        self.map.get(cell)
    }

    /// Get an exlusive reference to the element placed in `cell` if existing.
    pub fn get_mut(&mut self, cell: &Cell) -> Option<&mut T> {
        self.map.get_mut(cell)
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

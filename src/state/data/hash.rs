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

    /// Get a specific amount `count` of populated cells nearest to `cell`
    /// within `max_range`.
    ///
    /// The found cells will be written to `out` starting from index 0 to
    /// index `count`.
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
    ) -> Result<(), u32> {
        let mut rem = count;

        for i in 1..=max_range {
            let i = i as i32;
            for x in -i..i {
                for y in -i..i {
                    for z in -i..i {
                        let other = Cell {
                            x: cell.x + x,
                            y: cell.y + y,
                            z: cell.z + z,
                        };
                        if other == cell {
                            continue;
                        }
                        if self.map.get(&other).is_some() {
                            out.push(other);
                            rem -= 1;
                        }
                        if rem == 0 {
                            return Ok(());
                        }
                    }
                }
            }
        }

        if rem == 0 { Ok(()) } else { Err(rem) }
    }

    /// Get the nearest populated cell from a `cell` and its contents within
    /// `max_range`.
    ///
    /// # Returns
    /// * [`Ok`] containing the nearest populated cell and a reference to its
    ///   contents.
    /// * [`Err`] if there is no nearby populated cell; i.e. there no elements
    ///   present other than, maybe, the one in `cell`.
    pub fn nearest_cell(&self, cell: Cell, max_range: u32) -> Result<(Cell, &T), ()> {
        for i in 1..=max_range {
            let i = i as i32;
            for x in -i..i {
                for y in -i..i {
                    for z in -i..i {
                        let other = Cell {
                            x: cell.x + x,
                            y: cell.y + y,
                            z: cell.z + z,
                        };
                        if other == cell {
                            continue;
                        }
                        if let Some(element) = self.map.get(&other) {
                            return Ok((other, element));
                        }
                    }
                }
            }
        }

        Err(())
    }

    /// Get the nearest populated cell from a `point` and its contents within
    /// `max_range`.
    ///
    /// # Returns
    /// * [`Ok`] containing the nearest populated cell and a reference to its
    ///   contents.
    /// * [`Err`] if there is no nearby populated cell; i.e. there no elements
    ///   present other than, maybe, the one in the cell corresponding to
    ///   `point`.
    pub fn nearest_cell_point(&self, point: glam::Vec3, max_range: u32) -> Result<(Cell, &T), ()> {
        self.nearest_cell(self.cell_at(point), max_range)
    }

    /// Get the nearest populated cell from a `cell` and its contents within `max_range`.
    ///
    /// # Returns
    /// * [`Ok`] containing the nearest populated cell and an exclusive
    ///   reference to its contents.
    /// * [`Err`] if there is no nearby populated cell; i.e. there no elements
    ///   present other than, maybe, the one in `cell`.
    pub fn nearest_cell_mut(&mut self, cell: Cell, max_range: u32) -> Result<(Cell, &mut T), ()> {
        if let Ok((cell, _)) = self.nearest_cell(cell, max_range) {
            let e = self.map.get_mut(&cell).expect("nearest cell is populated");
            return Ok((cell, e));
        }
        Err(())
    }

    /// Get the nearest populated cell from a `point` and its contents within
    /// `max_range`.
    ///
    /// # Returns
    /// * [`Ok`] containing the nearest populated cell and an exclusive
    ///   reference to its contents.
    /// * [`Err`] if there is no nearby populated cell; i.e. there no elements
    ///   present other than, maybe, the one in the cell corresponding to
    ///   `point`.
    pub fn nearest_cell_point_mut(
        &mut self,
        point: glam::Vec3,
        max_range: u32,
    ) -> Result<(Cell, &mut T), ()> {
        self.nearest_cell_mut(self.cell_at(point), max_range)
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

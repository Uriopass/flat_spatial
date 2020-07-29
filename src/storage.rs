use crate::cell::GridCell;
use crate::shape::AABB;
use mint::Point2;
use std::collections::HashMap;

/// The storage trait, implement this if you want to use a custom point storage for the Grid.
pub trait Storage {
    type Idx: Copy + Eq;
    type IdxIter: Iterator<Item = Self::Idx>;

    fn new(cell_size: i32) -> Self;

    fn modify(&mut self, f: impl FnMut(&mut GridCell));

    fn cell_mut<IC>(&mut self, pos: Point2<f32>, on_ids_changed: IC) -> (Self::Idx, &mut GridCell)
    where
        IC: FnMut(&mut Self);
    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut GridCell;
    fn cell(&self, id: Self::Idx) -> Option<&GridCell>;

    fn cell_range(&self, ll: Self::Idx, ur: Self::Idx) -> Self::IdxIter;
    fn cell_id(&self, p: Point2<f32>) -> Self::Idx;

    fn cell_aabb(&self, id: Self::Idx) -> AABB;
}

/// DenseStorage stores cells in a Vec to be used for a Grid.
/// It implements the Storage trait.
pub struct DenseStorage {
    cell_size: i32,
    start_x: i32,
    start_y: i32,
    width: i32,
    height: i32,
    cells: Vec<GridCell>,
}

impl DenseStorage {
    /// Creates a new cell grid centered on zero with width and height defined by size.  
    ///
    /// Note that the size is counted in cells and not in absolute units (!)
    pub fn new_centered(cell_size: i32, size: i32) -> Self {
        Self::new_rect(cell_size, -size, -size, 2 * size, 2 * size)
    }

    /// Creates a new grid with a custom rect defining its boundaries.  
    ///
    /// Note that the coordinates are counted in cells and not in absolute units (!)
    pub fn new_rect(cell_size: i32, x: i32, y: i32, w: i32, h: i32) -> Self {
        assert!(
            cell_size > 0,
            "Cell size ({}) cannot be less than or equal to zero",
            cell_size
        );
        Self {
            start_x: x * cell_size,
            start_y: y * cell_size,
            cell_size,
            width: w,
            height: h,
            cells: (0..w * h).map(|_| Default::default()).collect(),
        }
    }

    pub fn cells(&self) -> &Vec<GridCell> {
        &self.cells
    }
}

impl Storage for DenseStorage {
    type Idx = usize;
    type IdxIter = DenseIter;

    fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            start_x: 0,
            start_y: 0,
            width: 0,
            height: 0,
            cells: vec![],
        }
    }

    fn modify(&mut self, f: impl FnMut(&mut GridCell)) {
        self.cells.iter_mut().for_each(f)
    }

    fn cell_mut<IC>(
        &mut self,
        pos: Point2<f32>,
        mut on_ids_changed: IC,
    ) -> (Self::Idx, &mut GridCell)
    where
        IC: FnMut(&mut Self),
    {
        debug_assert!(pos.x.is_finite());
        debug_assert!(pos.y.is_finite());

        if self.width == 0 && self.height == 0 {
            // First allocation, change start_x and start_y to match pos
            self.start_x = pos.x as i32 / self.cell_size * self.cell_size;
            self.start_y = pos.y as i32 / self.cell_size * self.cell_size;
        }
        let mut reallocate = false;

        let x = pos.x as i32;
        let y = pos.y as i32;

        if x <= self.start_x {
            let diff = 1 + (self.start_x - x) / self.cell_size;
            self.start_x -= self.cell_size * diff;
            self.width += diff;
            reallocate = true;
        }

        if y <= self.start_y {
            let diff = 1 + (self.start_y - y) / self.cell_size;
            self.start_y -= self.cell_size * diff;
            self.height += diff;
            reallocate = true;
        }

        let right = self.start_x + self.width as i32 * self.cell_size;
        if x >= right {
            self.width += 1 + (x - right) / self.cell_size;
            reallocate = true;
        }

        let up = self.start_y + self.height as i32 * self.cell_size;
        if y >= up {
            self.height += 1 + (y - up) / self.cell_size;
            if !reallocate {
                self.cells
                    .resize_with((self.width * self.height) as usize, GridCell::default);
            }
        }

        if reallocate {
            self.cells
                .resize_with((self.width * self.height) as usize, GridCell::default);
            on_ids_changed(self)
        }

        let id = self.cell_id(pos);
        (id, self.cell_mut_unchecked(id))
    }

    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut GridCell {
        &mut self.cells[id]
    }

    fn cell(&self, id: Self::Idx) -> Option<&GridCell> {
        self.cells.get(id)
    }

    fn cell_range(&self, ll: Self::Idx, ur: Self::Idx) -> Self::IdxIter {
        let w = self.width as usize;
        DenseIter {
            ur,
            diff: 1 + ur % w - ll % w,
            width: w,
            c: 0,
            cur: ll,
        }
    }

    fn cell_id(&self, pos: Point2<f32>) -> Self::Idx {
        ((pos.y as i32 - self.start_y) / self.cell_size * self.width
            + (pos.x as i32 - self.start_x) / self.cell_size) as usize
    }

    fn cell_aabb(&self, id: Self::Idx) -> AABB {
        let x = id as i32 % self.width;
        let y = id as i32 / self.width;

        let ll = Point2 {
            x: (self.start_x + x * self.cell_size) as f32,
            y: (self.start_y + y * self.cell_size) as f32,
        };

        let ur = Point2 {
            x: ll.x + self.cell_size as f32,
            y: ll.y + self.cell_size as f32,
        };

        AABB::new(ll, ur)
    }
}

pub struct DenseIter {
    ur: usize,
    width: usize,
    diff: usize,
    c: usize,
    cur: usize,
}

impl Iterator for DenseIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur > self.ur {
            return None;
        }

        let v = self.cur;
        self.c += 1;
        self.cur += 1;
        if self.c == self.diff {
            self.cur -= self.diff;
            self.cur += self.width;
        }
        Some(v)
    }
}

/// SparseStorage stores cells in a HashMap to be used in a Grid.
/// It is Sparse because cells are eagerly allocated, and cleaned when they are empty.
/// It implements the Storage trait.
pub struct SparseStorage {
    cell_size: i32,
    cells: HashMap<(i32, i32), GridCell>,
}

impl SparseStorage {
    pub fn cells(&self) -> &HashMap<(i32, i32), GridCell> {
        &self.cells
    }
}

impl Storage for SparseStorage {
    type Idx = (i32, i32);
    type IdxIter = XYRange;

    fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: Default::default(),
        }
    }

    fn modify(&mut self, mut f: impl FnMut(&mut GridCell)) {
        self.cells.retain(move |_, cell| {
            f(cell);
            !cell.objs.is_empty()
        });
    }

    // ids never change
    fn cell_mut<IC>(&mut self, pos: Point2<f32>, _on_ids_changed: IC) -> (Self::Idx, &mut GridCell)
    where
        IC: FnMut(&mut Self),
    {
        let id = self.cell_id(pos);
        (id, self.cells.entry(id).or_default())
    }

    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut GridCell {
        self.cells.get_mut(&id).unwrap()
    }

    fn cell(&self, id: Self::Idx) -> Option<&GridCell> {
        self.cells.get(&id)
    }

    fn cell_range(&self, (x1, y1): Self::Idx, (x2, y2): Self::Idx) -> Self::IdxIter {
        XYRange {
            x1,
            x2: x2 + 1,
            y2: y2 + 1,
            x: x1,
            y: y1,
        }
    }

    fn cell_id(&self, pos: Point2<f32>) -> Self::Idx {
        (pos.x as i32 / self.cell_size, pos.y as i32 / self.cell_size)
    }

    fn cell_aabb(&self, id: Self::Idx) -> AABB {
        let (x, y) = id;

        let ll = Point2 {
            x: (x * self.cell_size) as f32,
            y: (y * self.cell_size) as f32,
        };

        let ur = Point2 {
            x: ll.x + self.cell_size as f32,
            y: ll.y + self.cell_size as f32,
        };

        AABB::new(ll, ur)
    }
}

pub struct XYRange {
    x1: i32,
    x2: i32,
    y2: i32,
    x: i32,
    y: i32,
}

impl Iterator for XYRange {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y >= self.y2 {
            return None;
        }

        let v = (self.x, self.y);
        self.x += 1;
        if self.x == self.x2 {
            self.x = self.x1;
            self.y += 1;
        }

        Some(v)
    }
}

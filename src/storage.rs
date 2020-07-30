use crate::shape::AABB;
use mint::Point2;
use std::collections::HashMap;

/// The storage trait, implement this if you want to use a custom point storage for the Grid.
pub trait Storage<T> {
    type Idx: Copy + Eq;
    type IdxIter: Iterator<Item = Self::Idx>;

    fn new(cell_size: i32) -> Self;

    // f returns true if the cell is empty (which may lead to cleaning up)
    fn modify(&mut self, f: impl FnMut(&mut T) -> bool);

    fn cell_mut<IC>(&mut self, pos: Point2<f32>, on_ids_changed: IC) -> (Self::Idx, &mut T)
    where
        IC: FnMut(&mut Self);
    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut T;
    fn cell(&self, id: Self::Idx) -> Option<&T>;

    fn cell_range(&self, ll: Self::Idx, ur: Self::Idx) -> Self::IdxIter;
    fn cell_id(&self, p: Point2<f32>) -> Self::Idx;

    fn cell_aabb(&self, id: Self::Idx) -> AABB;
}

/// DenseStorage stores cells in a Vec to be used for a Grid.
/// It implements the Storage trait.
pub struct DenseStorage<T: Default> {
    cell_size: i32,
    start_x: i32,
    start_y: i32,
    width: i32,
    height: i32,
    cells: Vec<T>,
}

impl<T: Default> DenseStorage<T> {
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

    pub fn cells(&self) -> &Vec<T> {
        &self.cells
    }
}

impl<T: Default> Storage<T> for DenseStorage<T> {
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

    fn modify(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        self.cells.iter_mut().for_each(|x| {
            f(x);
        })
    }

    fn cell_mut<IC>(&mut self, pos: Point2<f32>, mut on_ids_changed: IC) -> (Self::Idx, &mut T)
    where
        IC: FnMut(&mut Self),
    {
        debug_assert!(pos.x.is_finite());
        debug_assert!(pos.y.is_finite());

        if self.width == 0 && self.height == 0 {
            // First allocation, change start_x and start_y to match pos
            self.start_x = pos.x as i32 / self.cell_size * self.cell_size;
            self.start_y = pos.y as i32 / self.cell_size * self.cell_size;
            self.width = 1;
            self.height = 1;
            self.cells = vec![T::default()];
        }
        let mut reallocate = false;

        let mut padleft = 0;
        let mut padright = 0;
        let mut paddown = 0;
        let mut padup = 0;

        let x = pos.x as i32;
        let y = pos.y as i32;

        let right = self.start_x + self.width as i32 * self.cell_size;
        let up = self.start_y + self.height as i32 * self.cell_size;

        if x <= self.start_x {
            padleft = 1 + (self.start_x - x) / self.cell_size;
            self.start_x -= self.cell_size * padleft;
            self.width += padleft;
            reallocate = true;
        } else if x >= right {
            padright = 1 + (x - right) / self.cell_size;
            self.width += padright;
            reallocate = true;
        }

        if y <= self.start_y {
            paddown = 1 + (self.start_y - y) / self.cell_size;
            self.start_y -= self.cell_size * paddown;
            self.height += paddown;
            reallocate = true;
            paddown = paddown;
        } else if y >= up {
            padup = 1 + (y - up) / self.cell_size;
            self.height += padup;
            if !reallocate {
                self.cells
                    .resize_with((self.width * self.height) as usize, T::default);
            }
        }

        if reallocate {
            let mut newvec = Vec::with_capacity((self.width * self.height) as usize);

            let oldh = self.height - paddown - padup;
            let oldw = self.width - padleft - padright;

            // use T::default to pad with new cells
            for _ in 0..paddown {
                newvec.extend((0..self.width).map(|_| T::default()))
            }
            for y in 0..oldh {
                newvec.extend((0..padleft).map(|_| T::default()));
                newvec.extend(
                    (0..oldw).map(|x| {
                        std::mem::take(self.cells.get_mut((y * oldw + x) as usize).unwrap())
                    }),
                );
                newvec.extend((0..padright).map(|_| T::default()))
            }
            for _ in 0..padup {
                newvec.extend((0..self.width).map(|_| T::default()))
            }

            self.cells = newvec;
            on_ids_changed(self)
        }

        let id = self.cell_id(pos);
        (id, self.cell_mut_unchecked(id))
    }

    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut T {
        &mut self.cells[id]
    }

    fn cell(&self, id: Self::Idx) -> Option<&T> {
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
        (((pos.y as i32 - self.start_y).max(0) / self.cell_size * self.width
            + (pos.x as i32 - self.start_x).max(0) / self.cell_size) as usize)
            .min(self.cells.len())
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
pub struct SparseStorage<T: Default> {
    cell_size: i32,
    cells: HashMap<(i32, i32), T>,
}

impl<T: Default> SparseStorage<T> {
    pub fn cells(&self) -> &HashMap<(i32, i32), T> {
        &self.cells
    }
}

impl<T: Default> Storage<T> for SparseStorage<T> {
    type Idx = (i32, i32);
    type IdxIter = XYRange;

    fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: Default::default(),
        }
    }

    fn modify(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        self.cells.retain(move |_, cell| !f(cell));
    }

    // ids never change
    fn cell_mut<IC>(&mut self, pos: Point2<f32>, _: IC) -> (Self::Idx, &mut T)
    where
        IC: FnMut(&mut Self),
    {
        let id = self.cell_id(pos);
        (id, self.cells.entry(id).or_default())
    }

    fn cell_mut_unchecked(&mut self, id: Self::Idx) -> &mut T {
        self.cells.entry(id).or_default()
    }

    fn cell(&self, id: Self::Idx) -> Option<&T> {
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

use crate::cell::{CellObject, GridCell};
use crate::grid::{GridHandle, GridObjects, ObjectState, StoreObject};
use mint::Point2;
use slotmap::SlotMap;
use std::collections::HashMap;

/// The storage trait, implement this if you want to use a custom point storage for the Grid.
pub trait Storage {
    type Idx: Copy;
    type IdxIter: Iterator<Item = Self::Idx>;

    fn new(cell_size: i32) -> Self;

    fn insert(&mut self, cell_id: Self::Idx, obj: CellObject);
    fn set_dirty(&mut self, cell_id: Self::Idx);
    fn maintain<O: Copy>(
        &mut self,
        objects: &mut SlotMap<GridHandle, StoreObject<O, Self::Idx>>,
        to_relocate: &mut Vec<(Self::Idx, CellObject)>,
    );

    fn check_resize<O: Copy>(
        &mut self,
        _pos: Point2<f32>,
        _objects: &mut GridObjects<O, Self::Idx>,
    ) {
    }
    fn get_cell(&self, id: Self::Idx) -> Option<&GridCell>;

    fn cell_range(&self, ll: Self::Idx, ur: Self::Idx) -> Self::IdxIter;

    fn get_cell_id(&self, p: Point2<f32>) -> Self::Idx;
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

    fn reallocate<T: Copy>(&mut self, objects: &mut GridObjects<T, usize>) {
        self.cells
            .resize_with((self.width * self.height) as usize, GridCell::default);

        for x in &mut self.cells {
            x.objs.clear();
            x.dirty = false;
        }

        for (id, obj) in objects {
            let cell_id = ((obj.pos.y as i32 - self.start_y) / self.cell_size * self.width
                + ((obj.pos.x as i32 - self.start_x) / self.cell_size))
                as usize;

            obj.cell_id = cell_id;
            obj.state = ObjectState::Unchanged;

            self.cells
                .get_mut(cell_id)
                .unwrap()
                .objs
                .push((id, obj.pos));
        }
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

    fn insert(&mut self, cell_id: Self::Idx, obj: (GridHandle, Point2<f32>)) {
        self.cells.get_mut(cell_id).unwrap().objs.push(obj);
    }

    fn set_dirty(&mut self, cell_id: Self::Idx) {
        self.cells.get_mut(cell_id).unwrap().dirty = true;
    }

    fn maintain<O: Copy>(
        &mut self,
        objects: &mut SlotMap<GridHandle, StoreObject<O, Self::Idx>>,
        to_relocate: &mut Vec<(Self::Idx, CellObject)>,
    ) {
        for (id, cell) in self.cells.iter_mut().filter(|x| x.dirty).enumerate() {
            cell.maintain(id, objects, to_relocate);
        }
    }

    fn check_resize<O: Copy>(&mut self, pos: Point2<f32>, objects: &mut GridObjects<O, Self::Idx>) {
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
            self.cells
                .resize_with((self.width * self.height) as usize, GridCell::default);
        }

        if reallocate {
            self.reallocate(objects);
        }
    }

    fn get_cell(&self, id: Self::Idx) -> Option<&GridCell> {
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

    fn get_cell_id(&self, pos: Point2<f32>) -> Self::Idx {
        ((pos.y as i32 - self.start_y) / self.cell_size * self.width
            + (pos.x as i32 - self.start_x) / self.cell_size) as usize
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

impl Storage for SparseStorage {
    type Idx = (i32, i32);
    type IdxIter = XYRange;

    fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: Default::default(),
        }
    }

    fn insert(&mut self, cell_id: Self::Idx, obj: CellObject) {
        self.cells.entry(cell_id).or_default().objs.push(obj);
    }

    fn set_dirty(&mut self, cell_id: Self::Idx) {
        self.cells.get_mut(&cell_id).unwrap().dirty = true;
    }

    fn maintain<O: Copy>(
        &mut self,
        objects: &mut SlotMap<GridHandle, StoreObject<O, Self::Idx>>,
        to_relocate: &mut Vec<(Self::Idx, CellObject)>,
    ) {
        self.cells.retain(|&id, cell| {
            if !cell.dirty {
                return true;
            }
            cell.maintain(id, objects, to_relocate);
            !cell.objs.is_empty()
        });
    }

    fn get_cell(&self, id: Self::Idx) -> Option<&GridCell> {
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

    fn get_cell_id(&self, pos: Point2<f32>) -> Self::Idx {
        (pos.x as i32 / self.cell_size, pos.y as i32 / self.cell_size)
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

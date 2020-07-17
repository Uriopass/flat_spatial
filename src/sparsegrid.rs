use mint::Point2;
use retain_mut::RetainMut;
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::collections::HashMap;

new_key_type! {
    /// This handle is used to modify the associated object or to update its position.
    /// It is returned by the _insert_ method of a SparseGrid.
    pub struct SparseGridHandle;
}

/// State of an object, maintain() updates the internals of the sparseGrid and resets this to Unchanged
#[derive(Clone, Copy, PartialEq, Eq)]
enum ObjectState {
    Unchanged,
    NewPos,
    Removed,
}

/// The actual object stored in the store
#[derive(Clone, Copy)]
struct StoreObject<O: Copy> {
    /// User-defined object to be associated with a value
    obj: O,
    state: ObjectState,
    pos: Point2<f32>,
    cell_id: PosIdx,
}

type PosIdx = (i32, i32);

type CellObject = (SparseGridHandle, Point2<f32>);

/// A single cell of the sparsegrid, can be empty
#[derive(Default, Clone)]
pub struct SparseGridCell {
    pub objs: Vec<CellObject>,
    pub dirty: bool,
}

/// SparseGrid is a point-based spatial partitioning structure that uses a HashMap of cells which acts as a
/// grid instead of a tree.
/// It is Sparse because cells containing inserted points are eagerly allocated,
/// and cleaned when they are empty.
///
/// ## Fast queries
/// In theory, SparseGrid should be faster than a quadtree/r-tree because it has no log costs
/// (calculating the cells around a point is trivial).  
/// However, it only works if the cell size is adapted to the problem, much like how a tree has to
/// be balanced to be efficient.  
///
/// ## Dynamicity
/// SparseGrid's big advantage is that it is dynamic, supporting lazy positions updates
/// and object removal in constant time. Once objects are in, there is almost no allocation happening.
///
/// Compare that to most immutable spatial partitioning structures out there, which pretty much require
/// to rebuild the entire tree every time.
///
/// A SlotMap is used for objects managing, adding a level of indirection between points and objects.
/// SlotMap is used because removal doesn't alter handles given to the user, while still having constant time access.
/// However it requires O to be copy, but SlotMap's author stated that they were working on a similar
/// map where Copy isn't required.
///
/// ## About object managment
///
/// In theory, you don't have to use the object managment directly, you can make your custom
/// Handle -> Object map by specifying "`()`" to be the object type.
/// _(This can be useful if your object is not Copy)_
/// Since `()` is zero sized, it should probably optimize away a lot of the object managment code.
///
/// ```rust
/// use flat_spatial::SparseGrid;
/// let mut g: SparseGrid<()> = SparseGrid::new(10);
/// let handle = g.insert([0.0, 0.0], ());
/// // Use handle however you want
/// ```
///
/// ## Examples
/// Here is a basic example that shows most of its capabilities:
/// ```rust
/// use flat_spatial::SparseGrid;
///
/// let mut g: SparseGrid<i32> = SparseGrid::new(10); // Creates a new grid with a cell width of 10 with an integer as extra data
/// let a = g.insert([0.0, 0.0], 0); // Inserts a new element with data: 0
///
/// {
///     let mut before = g.query_around([0.0, 0.0], 5.0).map(|(id, _pos)| id); // Queries for objects around a given point
///     assert_eq!(before.next(), Some(a));
///     assert_eq!(g.get(a).unwrap().1, &0);
/// }
/// let b = g.insert([0.0, 0.0], 1); // Inserts a new element, assigning a new unique and stable handle, with data: 1
///
/// g.remove(a); // Removes a value using the handle given by `insert`
///              // This won't have an effect until g.maintain() is called
///
/// g.maintain(); // Maintains the grid, which applies all removals and position updates (not needed for insertions)
///
/// assert_eq!(g.handles().collect::<Vec<_>>(), vec![b]); // We check that the "a" object has been removed
///
/// let after: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|(id, _pos)| id).collect(); // And that b is query-able
/// assert_eq!(after, vec![b]);
///
/// assert_eq!(g.get(b).unwrap().1, &1); // We also check that b still has his data associated
/// assert_eq!(g.get(a), None); // But that a doesn't exist anymore
/// ```
#[derive(Clone)]
pub struct SparseGrid<O: Copy> {
    cell_size: i32,
    cells: HashMap<PosIdx, SparseGridCell>,
    objects: SlotMap<SparseGridHandle, StoreObject<O>>,
    // Cache maintain vec to avoid allocating every time maintain is called
    to_relocate: Vec<(PosIdx, CellObject)>,
}

impl<O: Copy> SparseGrid<O> {
    /// Creates an empty grid.   
    /// The cell size should be about the same magnitude as your queries size.
    pub fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            objects: SlotMap::with_key(),
            to_relocate: vec![],
        }
    }

    /// Inserts a new object with a position and an associated object
    /// Returns the unique and stable handle to be used with get_obj
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// ```
    pub fn insert(&mut self, pos: impl Into<Point2<f32>>, obj: O) -> SparseGridHandle {
        let pos = pos.into();
        let cell_id = self.get_cell_id(pos);
        let handle = self.objects.insert(StoreObject {
            obj,
            state: ObjectState::Unchanged,
            pos,
            cell_id,
        });
        self.cells
            .entry(cell_id)
            .or_default()
            .objs
            .push((handle, pos));
        handle
    }

    /// Lazily sets the position of an object (if it is not marked for deletion).
    /// This won't be taken into account until maintain() is called.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.set_position(h, [3.0, 3.0]);
    /// ```
    pub fn set_position(&mut self, handle: SparseGridHandle, pos: impl Into<Point2<f32>>) {
        let pos = pos.into();
        let new_cell_id = self.get_cell_id(pos);

        let obj = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");
        let old_id = obj.cell_id;
        obj.cell_id = new_cell_id;
        obj.pos = pos;
        if obj.state != ObjectState::Removed {
            obj.state = ObjectState::NewPos
        }

        self.get_cell_mut(old_id).dirty = true;
    }

    /// Lazily removes an object from the grid.
    /// This won't be taken into account until maintain() is called.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.remove(h);
    /// ```
    pub fn remove(&mut self, handle: SparseGridHandle) {
        let st = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");

        st.state = ObjectState::Removed;
        let id = st.cell_id;
        self.get_cell_mut(id).dirty = true;
    }

    /// Maintains the world, updating all the positions (and moving them to corresponding cells)
    /// and removing necessary objects and empty cells.
    /// Runs in linear time O(N) where N is the number of objects.
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.remove(h);
    ///
    /// assert!(g.get(h).is_some());
    /// g.maintain();
    /// assert!(g.get(h).is_none());
    /// ```
    pub fn maintain(&mut self) {
        let Self {
            cells,
            objects,
            to_relocate,
            ..
        } = self;

        cells.retain(|&id, cell| {
            if !cell.dirty {
                return true;
            }

            cell.dirty = false;

            cell.objs.retain_mut(|(obj_id, obj_pos)| {
                let store_obj = &mut objects[*obj_id];
                match store_obj.state {
                    ObjectState::NewPos => {
                        store_obj.state = ObjectState::Unchanged;
                        *obj_pos = store_obj.pos;
                        let relocate = store_obj.cell_id != id;
                        if relocate {
                            to_relocate.push((store_obj.cell_id, (*obj_id, *obj_pos)));
                        }
                        !relocate
                    }
                    ObjectState::Removed => {
                        objects.remove(*obj_id);
                        false
                    }
                    _ => true,
                }
            });

            !cell.objs.is_empty()
        });

        for (cell_id, obj) in to_relocate.drain(..) {
            self.cells.entry(cell_id).or_default().objs.push(obj);
        }
    }

    /// Iterate over all handles
    pub fn handles(&self) -> impl Iterator<Item = SparseGridHandle> + '_ {
        self.objects.keys()
    }

    /// Read access to the cells
    pub fn cells(&self) -> impl Iterator<Item = &SparseGridCell> {
        self.cells.values()
    }

    /// Returns a reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<i32> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// assert_eq!(g.get(h), Some(([5.0, 3.0].into(), &42)));
    /// ```
    pub fn get(&self, id: SparseGridHandle) -> Option<(Point2<f32>, &O)> {
        self.objects.get(id).map(|x| (x.pos, &x.obj))
    }

    /// Returns a mutable reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    /// let mut g: SparseGrid<i32> = SparseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// *g.get_mut(h).unwrap().1 = 56;
    /// assert_eq!(g.get(h).unwrap().1, &56);
    /// ```    
    pub fn get_mut(&mut self, id: SparseGridHandle) -> Option<(Point2<f32>, &mut O)> {
        self.objects.get_mut(id).map(|x| (x.pos, &mut x.obj))
    }

    /// Queries for all objects around a position within a certain radius.
    /// Try to keep the radius asked and the cell size of similar magnitude for better performance.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    ///
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let a = g.insert([0.0, 0.0], ());
    ///
    /// let around: Vec<_> = g.query_around([2.0, 2.0], 5.0).map(|(id, _pos)| id).collect();
    ///
    /// assert_eq!(vec![a], around);
    /// ```
    #[rustfmt::skip]
    pub fn query_around(&self, pos: impl Into<Point2<f32>>, radius: f32) -> impl Iterator<Item=CellObject> + '_ {
        let pos = pos.into();
        let (x, y) = self.get_cell_id(pos);

        let rplus = (radius as i32) / self.cell_size;

        let x_diff = pos.x - (x * self.cell_size) as f32;
        let y_diff = pos.y - (y * self.cell_size) as f32;

        let remainder = radius - (rplus * self.cell_size) as f32;
        let left = x_diff < remainder;
        let bottom = y_diff < remainder;
        let right = self.cell_size as f32 - x_diff < remainder;
        let top = self.cell_size as f32 - y_diff < remainder;

        let x1 = x - rplus - left as i32;
        let y1 = y - rplus - bottom as i32;

        let x2 = x + rplus + right as i32;
        let y2 = y + rplus + top as i32;

        let radius2 = radius * radius;
        (y1..y2 + 1)
            .flat_map(move |y| (x1..x2 + 1).map(move |x| (x, y)))
            .flat_map(move |coords| self.cells.get(&coords))
            .flat_map(move |cell| cell.objs.iter())
            .filter(move |(_, pos_obj)| {
                let x = pos_obj.x - pos.x;
                let y = pos_obj.y - pos.y;
                x * x + y * y < radius2
            })
            .copied()
    }

    /// Queries for all objects in an aabb (aka a rect).
    /// Try to keep the rect's width/height of similar magnitudes to the cell size for better performance.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    ///
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let a = g.insert([0.0, 0.0], ());
    ///
    /// let around: Vec<_> = g.query_aabb([-1.0, -1.0], [1.0, 1.0]).map(|(id, _pos)| id).collect();
    ///
    /// assert_eq!(vec![a], around);
    /// ```
    #[rustfmt::skip]
    pub fn query_aabb(&self, aa: impl Into<Point2<f32>>, bb: impl Into<Point2<f32>>) -> impl Iterator<Item=CellObject> + '_ {
        let aa = aa.into();
        let bb = bb.into();

        let ll = [aa.x.min(bb.x), aa.y.min(bb.y)].into(); // lower left
        let ur = [aa.x.max(bb.x), aa.y.max(bb.y)].into(); // upper right

        let (x1, y1) = self.get_cell_id(ll);
        let (x2, y2) = self.get_cell_id(ur);

        (y1..y2 + 1)
            .flat_map(move |y| (x1..x2 + 1).map(move |x| (x, y)))
            .flat_map(move |coords| self.cells.get(&coords))
            .flat_map(move |cell| cell.objs.iter())
            .filter(move |(_, pos_obj)| {
                (pos_obj.x >= ll.x) && (pos_obj.x <= ur.x) &&
                    (pos_obj.y >= ll.y) && (pos_obj.y <= ur.y)
            })
            .copied()
    }

    /// Allows to look directly at what's in a cell covering a specific position.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::SparseGrid;
    ///
    /// let mut g: SparseGrid<()> = SparseGrid::new(10);
    /// let a = g.insert([2.0, 2.0], ());
    ///
    /// let around = g.get_cell([1.0, 1.0]).collect::<Vec<_>>();
    ///
    /// assert_eq!(vec![&(a, [2.0, 2.0].into())], around);
    /// ```
    pub fn get_cell(
        &mut self,
        pos: impl Into<mint::Point2<f32>>,
    ) -> impl Iterator<Item = &CellObject> {
        self.cells
            .get(&self.get_cell_id(pos.into()))
            .into_iter()
            .flat_map(|x| x.objs.iter())
    }

    /// Returns the number of objects currently available
    /// (removals that were not confirmed with maintain() are still counted)
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Checks if the grid contains objects or not
    /// (removals that were not confirmed with maintain() are still counted)
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    fn get_cell_mut(&mut self, id: PosIdx) -> &mut SparseGridCell {
        self.cells.get_mut(&id).expect("get_cell error")
    }

    #[inline]
    fn get_cell_id(&self, pos: Point2<f32>) -> PosIdx {
        (
            (pos.x as i32) / self.cell_size,
            (pos.y as i32) / self.cell_size,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SparseGrid;

    #[test]
    fn test_small_query() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([5.0, 0.0], ());
        let b = g.insert([11.0, 0.0], ());
        let c = g.insert([5.0, 8.0], ());
        assert_eq!(g.cells().count(), 2);

        let near: Vec<_> = g.query_around([6.0, 0.0], 2.0).map(|x| x.0).collect();
        assert_eq!(near, vec![a]);

        let mid: Vec<_> = g.query_around([8.0, 0.0], 4.0).map(|x| x.0).collect();
        assert!(mid.contains(&a));
        assert!(mid.contains(&b));

        let far: Vec<_> = g.query_around([6.0, 0.0], 10.0).map(|x| x.0).collect();
        assert!(far.contains(&a));
        assert!(far.contains(&b));
        assert!(far.contains(&c));
    }

    #[test]
    fn test_shrink() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([5.0, 0.0], ());
        g.remove(a);
        assert_eq!(g.cells().count(), 1);
        g.maintain();
        assert_eq!(g.cells().count(), 0);
    }

    #[test]
    fn test_big_query_around() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g.query_around([15.0, 0.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }

    #[test]
    fn test_big_query_rect() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g
            .query_aabb([5.5, 1.0], [15.5, -1.0])
            .map(|x| x.0)
            .collect();
        assert_eq!(q.len(), 10);
    }

    #[test]
    fn test_distance_test() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([3.0, 4.0], ());

        let far: Vec<_> = g.query_around([0.0, 0.0], 5.1).map(|x| x.0).collect();
        assert_eq!(far, vec![a]);

        let near: Vec<_> = g.query_around([0.0, 0.0], 4.9).map(|x| x.0).collect();
        assert_eq!(near, vec![]);
    }

    #[test]
    fn test_change_position() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.set_position(a, [30.0, 30.0]);
        g.maintain();

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![]);

        let after: Vec<_> = g.query_around([30.0, 30.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![a]);
    }

    #[test]
    fn test_remove() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.remove(a);
        let b = g.insert([0.0, 0.0], ());
        g.maintain();

        assert_eq!(g.handles().collect::<Vec<_>>(), vec![b]);

        let after: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![b]);
    }

    #[test]
    fn test_resize() {
        let mut g: SparseGrid<()> = SparseGrid::new(10);
        let a = g.insert([-1000.0, 0.0], ());

        let q: Vec<_> = g.query_around([-1000.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![a]);

        let b = g.insert([0.0, 1000.0], ());

        let q: Vec<_> = g.query_around([0.0, 1000.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![b]);
    }
}

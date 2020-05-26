use mint::Point2;
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::cmp::{max, min};

new_key_type! {
    /// This handle is used to modify the store object or to update the position
    pub struct DenseGridHandle;
}

/// State of an object, maintain() updates the internals of the gridstore and resets this to Unchanged
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
    cell_id: usize,
}

type CellObject = (DenseGridHandle, Point2<f32>);

/// A single cell of the store, can be empty
#[derive(Default, Clone)]
pub struct DenseGridCell {
    pub objs: Vec<CellObject>,
    pub dirty: bool,
}

/// DenseGrid is a point-based spatial partitioning structure that uses a simple Vec which acts as a
/// grid instead of a tree.
/// It is Dense because all cells within the bounding rectangle of the inserted points must be allocated,
/// even if they are empty.
///
/// ## Fast queries
/// In theory, DenseGrid should be faster than a quadtree/r-tree because it has no log costs
/// (calculating the cells around a point is trivial).  
/// However, it only works if the cell size is adapted to the problem, much like how a tree has to
/// be balanced to be efficient. It is also memory hungry since all empty cells have to be allocated.
///
/// ## Dynamicity
/// DenseGrid's big advantage is that it is dynamic, supporting lazy positions updates
/// and object removal in constant time. Once objects are in, there is almost no allocation happening
/// if the objects don't move too much. [^1]
///
/// Compare that to most immutable spatial partitioning structures out there, which pretty much require
/// to rebuild the entire tree every time.
///
/// A SlotMap is used for objects managing, adding a level of indirection between points and objects.
/// SlotMap is used because removal doesn't alter handles given to the user, while still having constant time access.
/// However it requires O to be copy, but SlotMap's author stated that they were working on a similar
/// map where Copy isn't required.
///
/// [^1]: If an object goes out of the boundaries, then the boundary has to grow. Therefore, all cells have
/// to be reallocated and all the points have to be reinserted.
/// This can be solved in constant time using a SparseGrid, which has yet to be implemented.
///  
///  
/// ## About object managment
///
/// In theory, you don't have to use the object managment directly, you can make your custom
/// Handle -> Object map by specifying "`()`" to be the object type.
/// _(This can be useful if your object is not Copy)_
/// Since `()` is zero sized, it should probably optimize away a lot of the object managment code.
///
/// ```rust
/// use flat_spatial::DenseGrid;
/// let mut g: DenseGrid<()> = DenseGrid::new(10);
/// let handle = g.insert([0.0, 0.0], ());
/// // Use handle however you want
/// ```
///
/// ## Examples
/// Here is a basic example that shows most of its capabilities:
/// ```rust
/// use flat_spatial::DenseGrid;
///
/// let mut g: DenseGrid<i32> = DenseGrid::new(10); // Creates a new grid with a cell width of 10 with an integer as extra data
/// let a = g.insert([0.0, 0.0], 0); // Inserts a new element with data: 0
///
/// {
///     let mut before = g.query_around([0.0, 0.0], 5.0).map(|(id, _pos)| id); // Queries for objects around a given point
///     assert_eq!(before.next(), Some(&a));
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
/// assert_eq!(after, vec![&b]);
///
/// assert_eq!(g.get(b).unwrap().1, &1); // We also check that b still has his data associated
/// assert_eq!(g.get(a), None); // But that a doesn't exist anymore
/// ```
///
/// Here is a bit more complicated example, which shows how it can be used in a video game
/// ```rust
/// use flat_spatial::DenseGrid;
///
/// // A structure has to be copy in order to be in a dense grid
/// #[derive(Copy, Clone)]
/// struct Car {
///     direction: [f32; 2],
/// }
///
/// // Creates the grid with cell size 10
/// let mut g: DenseGrid<Car> = DenseGrid::new(10);
///
/// // create objects in the range x: [-50..50], y: [-50..50]
/// for _ in 0..100 {
///     let pos = [100.0 * rand::random::<f32>() - 50.0, 100.0 * rand::random::<f32>() - 50.0];
///     let magn = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
///     g.insert(
///         pos,
///         Car {
///             direction: [-pos[0] / magn, -pos[1] / magn],
///         },
///     );
/// }
///
/// fn update_loop(g: &mut DenseGrid<Car>) {
///     let handles: Vec<_> = g.handles().collect();
///     // Handle collisions (remove on collide)
///     for &h in &handles {
///         let (pos, _car) = g.get(h).unwrap();
///         let mut collided = false;
///         for (other_h, other_pos) in g.query_around(pos, 8.0) {
///             if other_h == &h {
///                 continue;
///             }
///             if (other_pos.x - pos.x).powi(2) + (other_pos.y - pos.y).powi(2) < 2.0 * 2.0 {
///                 collided = true;
///                 break;
///             }
///         }
///         
///         if collided {
///             g.remove(h);
///         }
///     }
///
///     // Update positions
///     for &h in &handles {
///         let (pos, car) = g.get(h).unwrap();
///         g.set_position(h, [pos.x + car.direction[0], pos.y + car.direction[1]])
///     }
///
///     // Handle position updates and removals
///     g.maintain();
/// }
/// ```
///
/// ## Schema
/// Here is a schema showing a bit more visually how the structure works
///
/// ![schema](https://i.imgur.com/2rkQbxB.png)
#[derive(Clone)]
pub struct DenseGrid<O: Copy> {
    start_x: i32,
    start_y: i32,
    cell_size: i32,
    width: i32,
    height: i32,
    cells: Vec<DenseGridCell>,
    objects: SlotMap<DenseGridHandle, StoreObject<O>>,
    // Cache maintain vec to avoid allocating every time maintain is called
    to_relocate: Vec<(usize, CellObject)>,
}

impl<O: Copy> DenseGrid<O> {
    /// Creates an empty grid that will center itself on the first coordinate given.   
    /// The cell size should be about the same magnitude as your queries size.
    pub fn new(cell_size: i32) -> Self {
        Self::new_rect(cell_size, 0, 0, 0, 0)
    }

    /// Creates a new grid centered on zero with width and height defined by size.  
    /// The cell size should be about the same magnitude as your queries size.  
    ///
    /// Note that the size is counted in cells and not in absolute units (!)
    pub fn new_centered(cell_size: i32, size: i32) -> Self {
        Self::new_rect(cell_size, -size, -size, 2 * size, 2 * size)
    }

    /// Creates a new grid with a custom rect defining its boundaries.  
    /// The cell size should be about the same magnitude as your queries size.  
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
            cells: (0..w * h).map(|_| DenseGridCell::default()).collect(),
            objects: SlotMap::with_key(),
            to_relocate: vec![],
        }
    }

    /// Inserts a new object with a position and an associated object
    /// Returns the unique and stable handle to be used with get_obj
    /// May reallocate the grid if pos is out of the boundary
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// ```
    pub fn insert(&mut self, pos: impl Into<Point2<f32>>, obj: O) -> DenseGridHandle {
        let pos = pos.into();
        self.check_resize(pos);
        let cell_id = self.get_cell_id(pos);
        let handle = self.objects.insert(StoreObject {
            obj,
            state: ObjectState::Unchanged,
            pos,
            cell_id,
        });
        self.get_cell_mut(cell_id).objs.push((handle, pos));
        handle
    }

    /// Lazily sets the position of an object (if it is not marked for deletion).
    /// This won't be taken into account until maintain() is called.  
    /// May reallocate the grid if pos is out of the boundary.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.set_position(h, [3.0, 3.0]);
    /// ```
    pub fn set_position(&mut self, handle: DenseGridHandle, pos: impl Into<Point2<f32>>) {
        let pos = pos.into();
        self.check_resize(pos);
        let new_cell_id = self.get_cell_id(pos);

        let obj = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");
        let old_id = obj.cell_id;
        obj.cell_id = new_cell_id;
        obj.pos = pos;
        match obj.state {
            ObjectState::Removed => {}
            _ => obj.state = ObjectState::NewPos,
        }

        self.get_cell_mut(old_id).dirty = true;
    }

    /// Lazily removes an object from the store.
    /// This won't be taken into account until maintain() is called.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.remove(h);
    /// ```
    pub fn remove(&mut self, handle: DenseGridHandle) {
        let st = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");

        st.state = ObjectState::Removed;
        let id = st.cell_id;
        self.get_cell_mut(id).dirty = true;
    }

    /// Maintains the world, updating all the positions (and moving them to corresponding cells) and removing necessary objects.
    /// Runs in linear time O(C + O) where C is the number of cells and O the number of objects.
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], ());
    /// g.remove(h);
    ///
    /// assert!(g.get(h).is_some());
    /// g.maintain();
    /// assert!(g.get(h).is_none());
    /// ```
    pub fn maintain(&mut self) {
        let cells = &mut self.cells;
        let objects = &mut self.objects;
        let to_relocate = &mut self.to_relocate;

        for (id, cell) in cells.iter_mut().filter(|x| x.dirty).enumerate() {
            cell.dirty = false;

            for _ in my_drain_filter(&mut cell.objs, |(obj_id, obj_pos)| {
                let store_obj = objects.get_mut(*obj_id).unwrap();
                match store_obj.state {
                    ObjectState::NewPos => {
                        store_obj.state = ObjectState::Unchanged;
                        *obj_pos = store_obj.pos;
                        let relocate = store_obj.cell_id != id;
                        if relocate {
                            to_relocate.push((store_obj.cell_id, (*obj_id, *obj_pos)));
                        }
                        relocate
                    }
                    ObjectState::Removed => {
                        objects.remove(*obj_id);
                        true
                    }
                    _ => false,
                }
            }) {}
        }

        for (cell_id, obj) in to_relocate.drain(..) {
            self.cells[cell_id].objs.push(obj);
        }
    }

    /// Iterate over all handles
    pub fn handles<'a>(&'a self) -> impl Iterator<Item = DenseGridHandle> + 'a {
        self.objects.keys()
    }

    /// Read access to the cells
    pub fn cells(&self) -> &Vec<DenseGridCell> {
        &self.cells
    }

    /// Returns a reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<i32> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// assert_eq!(g.get(h), Some(([5.0, 3.0].into(), &42)));
    /// ```
    pub fn get(&self, id: DenseGridHandle) -> Option<(Point2<f32>, &O)> {
        self.objects.get(id).map(|x| (x.pos, &x.obj))
    }

    /// Returns a mutable reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    /// let mut g: DenseGrid<i32> = DenseGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// *g.get_mut(h).unwrap().1 = 56;
    /// assert_eq!(g.get(h).unwrap().1, &56);
    /// ```    
    pub fn get_mut(&mut self, id: DenseGridHandle) -> Option<(Point2<f32>, &mut O)> {
        self.objects.get_mut(id).map(|x| (x.pos, &mut x.obj))
    }

    /// Queries for all objects around a position within a certain radius.
    /// Try to keep the radius asked and the cell size of similar magnitude for better performance.
    /// 
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    ///
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let a = g.insert([0.0, 0.0], ());
    ///
    /// let around: Vec<_> = g.query_around([2.0, 2.0], 5.0).map(|(id, _pos)| id).collect();
    /// 
    /// assert_eq!(vec![&a], around);
    /// ```
    #[rustfmt::skip]
    pub fn query_around(&self, pos: impl Into<Point2<f32>>, radius: f32) -> impl Iterator<Item=&CellObject> {
        let pos = pos.into();
        let cell = self.get_cell_id(pos) as i32;

        let (w, h) = (self.width, self.height);
        let y = cell / w;
        let x = cell - y * w;

        let rplus = (radius as i32) / self.cell_size;

        let x_diff = pos.x - (self.start_x + x * self.cell_size) as f32;
        let y_diff = pos.y - (self.start_y + y * self.cell_size) as f32;

        let remainder = radius - (rplus * self.cell_size) as f32;
        let left = x_diff < remainder;
        let bottom = y_diff < remainder;
        let right = self.cell_size as f32 - x_diff < remainder;
        let top = self.cell_size as f32 - y_diff < remainder;

        let x1 = max(0, x - rplus - left as i32);
        let y1 = max(0, y - rplus - bottom as i32);

        let x2 = min(w - 1, x + rplus + right as i32);
        let y2 = min(h - 1, y + rplus + top as i32);

        let radius2 = radius * radius;
        (y1..y2 + 1).flat_map(move |y| {
            (x1..x2 + 1).flat_map(move |x| {
                let cell_id = y * self.width + x;
                // Safety: min and max boundaries just above
                //         Works because of invariant self.cells.len() == height * width 
                let cell = unsafe { &self.cells.get_unchecked(cell_id as usize) };
                cell.objs.iter().filter(move |(_, pos_obj)| {
                    let x = pos_obj.x - pos.x;
                    let y = pos_obj.y - pos.y;
                    x * x + y * y < radius2
                })
            })
        })
    }

    /// Queries for all objects in an aabb (aka a rect).
    /// Try to keep the rect's width/height of similar magnitudes to the cell size for better performance.
    /// 
    /// # Example
    /// ```rust
    /// use flat_spatial::DenseGrid;
    ///
    /// let mut g: DenseGrid<()> = DenseGrid::new(10);
    /// let a = g.insert([0.0, 0.0], ());
    ///
    /// let around: Vec<_> = g.query_aabb([-1.0, -1.0], [1.0, 1.0]).map(|(id, _pos)| id).collect();
    /// 
    /// assert_eq!(vec![&a], around);
    /// ```
    #[rustfmt::skip]
    pub fn query_aabb(&self, aa: impl Into<Point2<f32>>, bb: impl Into<Point2<f32>>) -> impl Iterator<Item=&CellObject> {
        let aa = aa.into();
        let bb = bb.into();

        let ll = [aa.x.min(bb.x), aa.y.min(bb.y)].into(); // lower left
        let ur = [aa.x.max(bb.x), aa.y.max(bb.y)].into(); // upper right

        let (w, h) = (self.width, self.height);

        let cell = self.get_cell_id(ll) as i32;
        let y1 = cell / w;
        let x1 = cell - y1 * w;

        let cell2 = self.get_cell_id(ur) as i32;
        let y2 = cell2 / w;
        let x2 = cell2 - y2 * w;

        let x1 = x1.max(0);
        let y1 = y1.max(0);

        let x2 = x2.min(w-1);
        let y2 = y2.min(h-1);

        (y1..y2 + 1).flat_map(move |y| {
            (x1..x2 + 1).flat_map(move |x| {
                let cell_id = y * self.width + x;
                // Safety: min and max boundaries just above
                //         Works because of invariant self.cells.len() == height * width 
                let cell = unsafe { &self.cells.get_unchecked(cell_id as usize) };
                cell.objs.iter().filter(move |(_, pos_obj)| {
                    (pos_obj.x >= ll.x) && (pos_obj.x <= ur.x) &&
                    (pos_obj.y >= ll.y) && (pos_obj.y <= ur.y)
                })
            })
        })
    }

    /// Returns the (x, y, width, height) tuple representing the current allocated rect
    pub fn get_rect(&self) -> (i32, i32, i32, i32) {
        (self.start_x, self.start_y, self.width, self.height)
    }

    fn check_resize(&mut self, pos: Point2<f32>) {
        debug_assert!(pos.x.is_finite());
        debug_assert!(pos.y.is_finite());

        if self.width == 0 && self.height == 0 {
            // First allocation, change start_x and start_y to match pos
            self.start_x = pos.x as i32 / self.cell_size;
            self.start_y = pos.y as i32 / self.cell_size;
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
                .resize_with((self.width * self.height) as usize, DenseGridCell::default);
        }

        if reallocate {
            self.reallocate();
        }
    }

    fn reallocate(&mut self) {
        self.cells
            .resize_with((self.width * self.height) as usize, DenseGridCell::default);

        for x in &mut self.cells {
            x.objs.clear();
            x.dirty = false;
        }

        for (id, obj) in &mut self.objects {
            let cell_id = Self::get_cell_id_raw(
                self.width as i32,
                self.start_x,
                self.start_y,
                self.cell_size,
                obj.pos,
            );
            obj.cell_id = cell_id;
            obj.state = ObjectState::Unchanged;

            self.cells
                .get_mut(cell_id)
                .unwrap()
                .objs
                .push((id, obj.pos));
        }
    }

    fn get_cell_mut(&mut self, id: usize) -> &mut DenseGridCell {
        self.cells.get_mut(id).expect("get_cell error")
    }

    fn get_cell_id(&self, pos: Point2<f32>) -> usize {
        Self::get_cell_id_raw(
            self.width as i32,
            self.start_x,
            self.start_y,
            self.cell_size,
            pos,
        )
    }

    fn get_cell_id_raw(
        width: i32,
        start_x: i32,
        start_y: i32,
        cell_size: i32,
        pos: Point2<f32>,
    ) -> usize {
        let i_x = (pos.x as i32 - start_x) / cell_size;
        let i_y = (pos.y as i32 - start_y) / cell_size;
        (i_y * width + i_x) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::DenseGrid;

    #[test]
    fn test_small_query() {
        let mut g: DenseGrid<()> = DenseGrid::new(10);
        let a = g.insert([5.0, 0.0], ());
        let b = g.insert([11.0, 0.0], ());
        let c = g.insert([5.0, 8.0], ());

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
    fn test_big_query_around() {
        let mut g: DenseGrid<()> = DenseGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g.query_around([15.0, 0.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }

    #[test]
    fn test_big_query_rect() {
        let mut g: DenseGrid<()> = DenseGrid::new(10);

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
        let mut g: DenseGrid<()> = DenseGrid::new(10);
        let a = g.insert([3.0, 4.0], ());

        let far: Vec<_> = g.query_around([0.0, 0.0], 5.1).map(|x| x.0).collect();
        assert_eq!(far, vec![a]);

        let near: Vec<_> = g.query_around([0.0, 0.0], 4.9).map(|x| x.0).collect();
        assert_eq!(near, vec![]);
    }

    #[test]
    fn test_change_position() {
        let mut g: DenseGrid<()> = DenseGrid::new(10);
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
        let mut g: DenseGrid<()> = DenseGrid::new(10);
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
        let mut g: DenseGrid<()> = DenseGrid::new(10);
        let a = g.insert([-1000.0, 0.0], ());

        let q: Vec<_> = g.query_around([-1000.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![a]);

        let b = g.insert([0.0, 1000.0], ());

        let q: Vec<_> = g.query_around([0.0, 1000.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![b]);
    }
}

// Taken from stdlib since it's not stable yet (but it has been 2 years and there's bikeshedding so I'm tired of waiting)

fn my_drain_filter<T, F>(vec: &mut Vec<T>, filter: F) -> MyDrainFilter<T, F>
where
    F: FnMut(&mut T) -> bool,
{
    let old_len = vec.len();

    // Guard against us getting leaked (leak amplification)
    unsafe {
        vec.set_len(0);
    }

    MyDrainFilter {
        vec,
        idx: 0,
        del: 0,
        old_len,
        pred: filter,
    }
}

/// An iterator produced by calling `drain_filter` on Vec.
#[derive(Debug)]
struct MyDrainFilter<'a, T: 'a, F>
where
    F: FnMut(&mut T) -> bool,
{
    vec: &'a mut Vec<T>,
    idx: usize,
    del: usize,
    old_len: usize,
    pred: F,
}

impl<'a, T, F> Iterator for MyDrainFilter<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        unsafe {
            while self.idx != self.old_len {
                let i = self.idx;
                self.idx += 1;
                let v = std::slice::from_raw_parts_mut(self.vec.as_mut_ptr(), self.old_len);
                if (self.pred)(&mut v[i]) {
                    self.del += 1;
                    return Some(std::ptr::read(&v[i]));
                } else if self.del > 0 {
                    v.swap(i - self.del, i);
                }
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.old_len - self.idx))
    }
}

impl<'a, T, F> Drop for MyDrainFilter<'a, T, F>
where
    F: FnMut(&mut T) -> bool,
{
    fn drop(&mut self) {
        for _ in self.by_ref() {}

        unsafe {
            self.vec.set_len(self.old_len - self.del);
        }
    }
}

use crate::cell::ShapeGridCell;
use crate::shape::{Circle, Intersect, Shape};
use crate::storage::{cell_range, SparseStorage, Storage};
use mint::Point2;
use slotmap::new_key_type;
use slotmap::SlotMap;
use std::collections::HashSet;

pub type ShapeGridObjects<O, S> = SlotMap<ShapeGridHandle, StoreObject<O, S>>;

new_key_type! {
    /// This handle is used to modify the associated object or to update its position.
    /// It is returned by the _insert_ method of a ShapeGrid.
    pub struct ShapeGridHandle;
}

/// The actual object stored in the store
#[derive(Clone, Copy)]
pub struct StoreObject<O: Copy, S: Shape> {
    /// User-defined object to be associated with a value
    obj: O,
    pub shape: S,
}

/// ShapeGrid is a generic shape-based spatial partitioning structure that uses a generic storage of cells which acts as a
/// grid instead of a tree.
///
/// ## Fast queries
/// In theory, ShapeGrid should be faster than a quadtree/r-tree because it has no log costs
/// (calculating the cells around a point is trivial).  
/// However, it only works if the cell size is adapted to the problem, much like how a tree has to
/// be balanced to be efficient.  
///
/// ## Dynamicity
/// ShapeGrid's allows eager removals and position updates, however for big shapes (spanning many cells)
/// this can be expensive, so beware.
///
/// Use this grid for mostly static objects with the occasional removal/position update if needed.
///
/// A SlotMap is used for objects managing, adding a level of indirection between shapes and objects.
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
/// use flat_spatial::ShapeGrid;
/// use flat_spatial::shape::Circle;
///
/// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
/// let handle = g.insert(Circle {
///     center: [0.0, 0.0].into(),
///     radius: 3.0,
/// }, ());
/// // Use handle however you want
/// ```
///
/// ## Examples
/// Here is a basic example that shows most of its capabilities:
/// ```rust
/// use flat_spatial::ShapeGrid;
/// use flat_spatial::shape::Circle;
///
/// let mut g: ShapeGrid<i32, Circle> = ShapeGrid::new(10); // Creates a new grid with a cell width of 10 with an integer as extra data
/// let a = g.insert(Circle {
///      center: [2.0, 2.0].into(),
///      radius: 3.0,
/// }, 0); // Inserts a new circle with data: 0
///
/// {
///     let mut before = g.query_around([0.0, 0.0], 5.0).map(|(id, _shape, _obj)| id); // Queries for circles intersecting around a given point
///     assert_eq!(before.next(), Some(a));
///     assert_eq!(g.get(a).unwrap().1, &0);
/// }
/// let b = g.insert(Circle {
///      center: [1.0, 1.0].into(),
///      radius: 3.0,
/// }, 1); // Inserts a new element, assigning a new unique and stable handle, with data: 1
///
/// g.remove(a); // Removes a value using the handle given by `insert`
///
/// assert_eq!(g.handles().collect::<Vec<_>>(), vec![b]); // We check that the "a" object has been removed
///
/// let after: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|(id, _shape, _obj)| id).collect(); // And that b is query-able
/// assert_eq!(after, vec![b]);
///
/// assert_eq!(g.get(b).unwrap().1, &1); // We also check that b still has his data associated
/// assert!(g.get(a).is_none()); // But that a doesn't exist anymore
/// ```
#[derive(Clone)]
pub struct ShapeGrid<O: Copy, S: Shape, ST: Storage<ShapeGridCell> = SparseStorage<ShapeGridCell>> {
    storage: ST,
    objects: ShapeGridObjects<O, S>,
}

impl<S: Shape, ST: Storage<ShapeGridCell>, O: Copy> ShapeGrid<O, S, ST> {
    /// Creates an empty grid.   
    /// The cell size should be about the same magnitude as your queries size.
    pub fn new(cell_size: i32) -> Self {
        Self {
            storage: ST::new(cell_size),
            objects: SlotMap::with_key(),
        }
    }

    /// Creates an empty grid.   
    /// The cell size should be about the same magnitude as your queries size.
    pub fn with_storage(st: ST) -> Self {
        Self {
            storage: st,
            objects: SlotMap::with_key(),
        }
    }

    fn cells_apply(storage: &mut ST, shape: &S, f: impl Fn(&mut ShapeGridCell, bool)) {
        let bbox = shape.bbox();
        let ll = storage.cell_mut(bbox.ll).0;
        let ur = storage.cell_mut(bbox.ur).0;
        for id in cell_range(ll, ur) {
            if !shape.intersects(storage.cell_aabb(id)) {
                continue;
            }
            f(storage.cell_mut_unchecked(id), ll == ur)
        }
    }

    /// Inserts a new object with a position and an associated object
    /// Returns the unique and stable handle to be used with get_obj
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    /// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
    /// let h = g.insert(Circle {
    ///      center: [2.0, 2.0].into(),
    ///      radius: 3.0,
    /// }, ());
    /// ```
    pub fn insert(&mut self, shape: S, obj: O) -> ShapeGridHandle {
        let Self {
            storage, objects, ..
        } = self;

        let h = objects.insert(StoreObject { obj, shape });
        Self::cells_apply(storage, &shape, |cell, sing_cell| {
            cell.objs.push((h, sing_cell));
        });
        h
    }

    /// Updates the shape of an object.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    /// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
    /// let h = g.insert(Circle {
    ///      center: [2.0, 2.0].into(),
    ///      radius: 3.0,
    /// }, ());
    ///
    /// g.set_shape(h, Circle {
    ///      center: [61.0, 35.0].into(),
    ///      radius: 8.0,
    /// });
    /// ```
    pub fn set_shape(&mut self, handle: ShapeGridHandle, shape: S) {
        let obj = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");

        let storage = &mut self.storage;

        Self::cells_apply(storage, &obj.shape, |cell, _| {
            let p = match cell.objs.iter().position(|(x, _)| *x == handle) {
                Some(x) => x,
                None => return,
            };
            cell.objs.swap_remove(p);
        });

        Self::cells_apply(storage, &shape, |cell, sing_cell| {
            cell.objs.push((handle, sing_cell))
        });

        obj.shape = shape;
    }

    /// Removes an object from the grid.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    /// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
    /// let h = g.insert(Circle {
    ///      center: [2.0, 2.0].into(),
    ///      radius: 3.0,
    /// }, ());
    /// g.remove(h);
    /// ```
    pub fn remove(&mut self, handle: ShapeGridHandle) {
        let st = self
            .objects
            .remove(handle)
            .expect("Object not in grid anymore");

        let storage = &mut self.storage;
        Self::cells_apply(storage, &st.shape, |cell, _| {
            let p = match cell.objs.iter().position(|(x, _)| *x == handle) {
                Some(x) => x,
                None => return,
            };
            cell.objs.swap_remove(p);
        });
    }

    /// Iterate over all handles
    pub fn handles(&self) -> impl Iterator<Item = ShapeGridHandle> + '_ {
        self.objects.keys()
    }

    /// Iterate over all objects
    pub fn objects(&self) -> impl Iterator<Item = &O> + '_ {
        self.objects.values().map(|x| &x.obj)
    }

    /// Returns a reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    /// let mut g: ShapeGrid<i32, [f32; 2]> = ShapeGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// assert_eq!(g.get(h), Some((&[5.0, 3.0], &42)));
    /// ```
    pub fn get(&self, id: ShapeGridHandle) -> Option<(&S, &O)> {
        self.objects.get(id).map(|x| (&x.shape, &x.obj))
    }

    /// Returns a mutable reference to the associated object and its position, using the handle.  
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    /// let mut g: ShapeGrid<i32, [f32; 2]> = ShapeGrid::new(10);
    /// let h = g.insert([5.0, 3.0], 42);
    /// *g.get_mut(h).unwrap().1 = 56;
    /// assert_eq!(g.get(h).unwrap().1, &56);
    /// ```    
    pub fn get_mut(&mut self, id: ShapeGridHandle) -> Option<(&S, &mut O)> {
        self.objects.get_mut(id).map(|x| (&x.shape, &mut x.obj))
    }

    /// The underlying storage
    pub fn storage(&self) -> &ST {
        &self.storage
    }

    /// Queries for objects intersecting a given shape.
    ///
    /// # Example
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    ///
    /// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
    /// let a = g.insert(Circle {
    ///      center: [2.0, 2.0].into(),
    ///      radius: 3.0,
    /// }, ());
    /// let b = g.insert(Circle {
    ///      center: [5.0, 2.0].into(),
    ///      radius: 3.0,
    /// }, ());
    ///
    /// let around: Vec<_> = g.query(Circle {
    ///      center: [0.0, 0.0].into(),
    ///      radius: 10.0,
    /// }).map(|(id, _shape, _obj)| id).collect();
    ///
    /// assert_eq!(vec![a, b], around);
    /// ```
    pub fn query<QS: Shape + Intersect<S> + 'static>(
        &self,
        shape: QS,
    ) -> impl Iterator<Item = (ShapeGridHandle, &S, &O)> + '_ {
        self.query_broad(shape)
            .map(move |h| {
                let obj = &self.objects[h];
                (h, &obj.shape, &obj.obj)
            })
            .filter(move |&(_, x, _)| shape.intersects(*x))
    }

    /// Queries for all objects in the cells intersecting the given shape
    ///
    /// # Exampley
    /// ```rust
    /// use flat_spatial::{ShapeGrid, shape::Circle};
    ///
    /// let mut g: ShapeGrid<(), Circle> = ShapeGrid::new(10);
    /// let a = g.insert(Circle {
    ///      center: [5.0, 5.0].into(),
    ///      radius: 1.0,
    /// }, ());
    ///
    /// let around: Vec<_> = g.query_broad(Circle {
    ///      center: [0.0, 0.0].into(),
    ///      radius: 1.0,
    /// }).collect();
    ///
    /// assert_eq!(vec![a], around); // a is given even if it doesn't intersect, because this only looks at the cells
    /// ```
    pub fn query_broad<QS: Shape + 'static>(
        &self,
        shape: QS,
    ) -> impl Iterator<Item = ShapeGridHandle> + '_ {
        let bbox = shape.bbox();
        let storage = &self.storage;

        let ll_id = storage.cell_id(bbox.ll);
        let ur_id = storage.cell_id(bbox.ur);

        let iter = cell_range(ll_id, ur_id)
            .filter(move |&id| shape.intersects(storage.cell_aabb(id)))
            .flat_map(move |id| storage.cell(id))
            .flat_map(|x| x.objs.iter().copied());

        if ll_id == ur_id {
            QueryIter::Simple(iter)
        } else {
            QueryIter::Dedup(HashSet::with_capacity(5), iter)
        }
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
}

impl<S: Shape, ST: Storage<ShapeGridCell>, O: Copy> ShapeGrid<O, S, ST>
where
    Circle: Intersect<S>,
{
    /// Queries for objects around a point, same as querying a circle at pos with a given radius.
    pub fn query_around(
        &self,
        pos: impl Into<Point2<f32>>,
        radius: f32,
    ) -> impl Iterator<Item = (ShapeGridHandle, &S, &O)> + '_ {
        self.query(Circle {
            center: pos.into(),
            radius,
        })
    }
}

enum QueryIter<T: Iterator<Item = (ShapeGridHandle, bool)>> {
    Simple(T),
    Dedup(HashSet<ShapeGridHandle>, T),
}

impl<T: Iterator<Item = (ShapeGridHandle, bool)>> Iterator for QueryIter<T> {
    type Item = ShapeGridHandle;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            QueryIter::Simple(x) => x.next().map(|(x, _)| x),
            QueryIter::Dedup(seen, x) => loop {
                let (v, sing_cell) = x.next()?;
                if sing_cell {
                    return Some(v);
                }
                if seen.insert(v) {
                    return Some(v);
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shape::{Circle, AABB};
    use crate::storage::Storage;
    use crate::DenseShapeGrid;

    #[test]
    fn test_small_query() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);
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
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g.query_around([15.0, 0.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }

    #[test]
    fn test_big_query_rect() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g
            .query(AABB::new([5.5, 1.0].into(), [15.5, -1.0].into()))
            .map(|x| x.0)
            .collect();
        assert_eq!(q.len(), 10);
    }

    #[test]
    fn test_distance_test() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);
        let a = g.insert([3.0, 4.0], ());

        let far: Vec<_> = g.query_around([0.0, 0.0], 5.1).map(|x| x.0).collect();
        assert_eq!(far, vec![a]);

        let near: Vec<_> = g.query_around([0.0, 0.0], 4.9).map(|x| x.0).collect();
        assert_eq!(near, vec![]);
    }

    #[test]
    fn test_change_position() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.set_shape(a, [30.0, 30.0]);

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![]);

        let after: Vec<_> = g.query_around([30.0, 30.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![a]);
    }

    #[test]
    fn test_no_cell() {
        let mut g: DenseShapeGrid<(), Circle> = DenseShapeGrid::new(10);
        g.insert(
            Circle {
                center: [15.0, 15.0].into(),
                radius: 6.0,
            },
            (),
        );

        let s = g.storage();
        assert!(s
            .cell(s.cell_id([1.0, 1.0].into()))
            .unwrap()
            .objs
            .is_empty());
    }

    #[test]
    fn test_circle_inter() {
        let c = Circle {
            center: [15.0, 15.0].into(),
            radius: 6.0,
        };
        let mut g: DenseShapeGrid<(), Circle> = DenseShapeGrid::new(10);
        let a = g.insert(c, ());

        assert_eq!(
            g.query(Circle {
                center: [5.0, 5.0].into(),
                radius: 6.0,
            })
            .count(),
            0
        );

        assert_eq!(
            g.query(Circle {
                center: [5.0, 5.0].into(),
                radius: 10.0,
            })
            .next()
            .map(|x| x.0),
            Some(a)
        );
    }

    #[test]
    fn test_remove() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.remove(a);
        let b = g.insert([0.0, 0.0], ());

        assert_eq!(g.handles().collect::<Vec<_>>(), vec![b]);

        let after: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![b]);
    }

    #[test]
    fn test_resize() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);
        let a = g.insert([-1000.0, 0.0], ());

        let q: Vec<_> = g.query_around([-1000.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![a]);

        let b = g.insert([0.0, 1000.0], ());

        let q: Vec<_> = g.query_around([0.0, 1000.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![b]);
    }

    #[test]
    fn test_big_query_around_vert() {
        let mut g: DenseShapeGrid<(), [f32; 2]> = DenseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([0.0, i as f32], ());
        }

        let q: Vec<_> = g.query_around([0.0, 15.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }
}

#[cfg(test)]
mod testssparse {
    use crate::shape::{Circle, AABB};
    use crate::storage::Storage;
    use crate::SparseShapeGrid;
    use mint::Point2;

    #[test]
    fn test_rand() {
        let mut g: SparseShapeGrid<(), AABB> = SparseShapeGrid::new(50);

        fastrand::seed(0);
        for _ in 0..1000 {
            let aabb = AABB::new(
                Point2 {
                    x: fastrand::f32() * 1000.0 - 500.0,
                    y: fastrand::f32() * 1000.0 - 500.0,
                },
                Point2 {
                    x: fastrand::f32() * 1000.0 - 500.0,
                    y: fastrand::f32() * 1000.0 - 500.0,
                },
            );

            //dbg!(g.storage.cell_aabb(g.storage.cell_id(aabb.ll)));

            //dbg!(aabb);

            let h = g.insert(aabb, ());
            assert_eq!(
                g.query_around(aabb.ll, 0.001)
                    .map(|(a, _, _)| a)
                    .collect::<Vec<_>>(),
                vec![h]
            );
            assert_eq!(
                g.query_around(aabb.ur, 0.001)
                    .map(|(a, _, _)| a)
                    .collect::<Vec<_>>(),
                vec![h]
            );
            g.remove(h);
        }
    }

    #[test]
    fn test_big_query_around_vert() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([0.0, i as f32], ());
        }

        let q: Vec<_> = g.query_around([0.0, 15.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }

    #[test]
    fn test_no_cell() {
        let mut g: SparseShapeGrid<(), Circle> = SparseShapeGrid::new(10);
        g.insert(
            Circle {
                center: [15.0, 15.0].into(),
                radius: 6.0,
            },
            (),
        );

        let s = g.storage();
        assert!(s
            .cell(s.cell_id([1.0, 1.0].into()))
            .unwrap()
            .objs
            .is_empty());
    }

    #[test]
    fn test_circle_inter() {
        let c = Circle {
            center: [15.0, 15.0].into(),
            radius: 6.0,
        };
        let mut g: SparseShapeGrid<(), Circle> = SparseShapeGrid::new(10);
        let a = g.insert(c, ());

        assert_eq!(
            g.query(Circle {
                center: [5.0, 5.0].into(),
                radius: 6.0,
            })
            .count(),
            0
        );

        assert_eq!(
            g.query(Circle {
                center: [5.0, 5.0].into(),
                radius: 10.0,
            })
            .next()
            .map(|x| x.0),
            Some(a)
        );
    }

    #[test]
    fn test_small_query() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);
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
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g.query_around([15.0, 0.0], 9.5).map(|x| x.0).collect();
        assert_eq!(q.len(), 19); // 1 middle, 8 left, 8 right
    }

    #[test]
    fn test_big_query_rect() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);

        for i in 0..100 {
            g.insert([i as f32, 0.0], ());
        }

        let q: Vec<_> = g
            .query(AABB::new([5.5, 1.0].into(), [15.5, -1.0].into()))
            .map(|x| x.0)
            .collect();
        assert_eq!(q.len(), 10);
    }

    #[test]
    fn test_distance_test() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);
        let a = g.insert([3.0, 4.0], ());

        let far: Vec<_> = g.query_around([0.0, 0.0], 5.1).map(|x| x.0).collect();
        assert_eq!(far, vec![a]);

        let near: Vec<_> = g.query_around([0.0, 0.0], 4.9).map(|x| x.0).collect();
        assert_eq!(near, vec![]);
    }

    #[test]
    fn test_change_position() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.set_shape(a, [30.0, 30.0]);

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![]);

        let after: Vec<_> = g.query_around([30.0, 30.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![a]);
    }

    #[test]
    fn test_remove() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);
        let a = g.insert([0.0, 0.0], ());

        let before: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(before, vec![a]);

        g.remove(a);
        let b = g.insert([0.0, 0.0], ());

        assert_eq!(g.handles().collect::<Vec<_>>(), vec![b]);

        let after: Vec<_> = g.query_around([0.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(after, vec![b]);
    }

    #[test]
    fn test_resize() {
        let mut g: SparseShapeGrid<(), [f32; 2]> = SparseShapeGrid::new(10);
        let a = g.insert([-1000.0, 0.0], ());

        let q: Vec<_> = g.query_around([-1000.0, 0.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![a]);

        let b = g.insert([0.0, 1000.0], ());

        let q: Vec<_> = g.query_around([0.0, 1000.0], 5.0).map(|x| x.0).collect();
        assert_eq!(q, vec![b]);
    }
}

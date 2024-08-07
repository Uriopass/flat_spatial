use crate::cell::AABBGridCell;
use crate::storage::{cell_range, SparseStorage};
use crate::AABB;
use slotmapd::{new_key_type, SlotMap};

pub type AABBGridObjects<O, AB> = SlotMap<AABBGridHandle, StoreObject<O, AB>>;

new_key_type! {
    /// This handle is used to modify the associated object or to update its position.
    /// It is returned by the _insert_ method of a AABBGrid.
    pub struct AABBGridHandle;
}

/// The actual object stored in the store
#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoreObject<O: Copy, AB: AABB> {
    /// User-defined object to be associated with a value
    pub obj: O,
    pub aabb: AB,
}

/// `AABBGrid` is a generic aabb-based spatial partitioning structure that uses a generic storage of cells which acts as a
/// grid instead of a tree.
///
/// ## Fast queries
/// In theory, `AABBGrid` should be faster than a quadtree/r-tree because it has no log costs
/// (calculating the cells around a point is trivial).  
/// However, it only works if the cell size is adapted to the problem, much like how a tree has to
/// be balanced to be efficient.  
///
/// ## Dynamicity
/// `AABBGrid's` allows eager removals and position updates, however for big aabbs (spanning many cells)
/// this can be expensive, so beware.
///
/// Use this grid for mostly static objects with the occasional removal/position update if needed.
///
/// A `SlotMap` is used for objects managing, adding a level of indirection between aabbs and objects.
/// `SlotMap` is used because removal doesn't alter handles given to the user, while still having constant time access.
/// However it requires O to be copy, but `SlotMap's` author stated that they were working on a similar
/// map where Copy isn't required.
///
/// ## About object management
///
/// In theory, you don't have to use the object management directly, you can make your custom
/// Handle -> Object map by specifying "`()`" to be the object type.
/// _(This can be useful if your object is not Copy)_
/// Since `()` is zero sized, it should probably optimize away a lot of the object management code.
///
/// ```rust
/// use flat_spatial::AABBGrid;
/// use euclid::default::Rect;
///
/// let mut g: AABBGrid<(), Rect<f32>> = AABBGrid::new(10);
/// let handle = g.insert(Rect::new([0.0, 0.0].into(), [10.0, 10.0].into()), ());
/// // Use handle however you want
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AABBGrid<O: Copy, AB: AABB> {
    storage: SparseStorage<AABBGridCell>,
    objects: AABBGridObjects<O, AB>,
}

impl<O: Copy, AB: AABB> AABBGrid<O, AB> {
    /// Creates an empty grid.
    /// The cell size should be about the same magnitude as your queries size.
    pub fn new(cell_size: i32) -> Self {
        Self {
            storage: SparseStorage::new(cell_size),
            objects: AABBGridObjects::default(),
        }
    }

    /// Clears the grid.
    pub fn clear(&mut self) -> impl Iterator<Item = (AB, O)> {
        self.storage = SparseStorage::new(self.storage.cell_size());
        let objs = std::mem::take(&mut self.objects);
        objs.into_iter().map(|(_, o)| (o.aabb, o.obj))
    }

    /// Inserts a new object with a position and an associated object
    /// Returns the unique and stable handle to be used with `get_obj`
    pub fn insert(&mut self, aabb: AB, obj: O) -> AABBGridHandle {
        let Self {
            storage, objects, ..
        } = self;

        let h = objects.insert(StoreObject { obj, aabb });
        cells_apply(storage, &aabb, |cell, sing_cell| {
            cell.objs.push((h, sing_cell));
        });
        h
    }

    /// Updates the aabb of an object.
    pub fn set_aabb(&mut self, handle: AABBGridHandle, aabb: AB) {
        let obj = self
            .objects
            .get_mut(handle)
            .expect("Object not in grid anymore");

        let storage = &mut self.storage;

        let old_ll = storage.cell_mut(obj.aabb.ll()).0;
        let old_ur = storage.cell_mut(obj.aabb.ur()).0;

        let ll = storage.cell_mut(aabb.ll()).0;
        let ur = storage.cell_mut(aabb.ur()).0;

        obj.aabb = aabb;

        if old_ll == ll && old_ur == ur {
            return;
        }

        for id in cell_range(old_ll, old_ur) {
            let cell = storage.cell_mut_unchecked(id);
            let p = match cell.objs.iter().position(|(x, _)| *x == handle) {
                Some(x) => x,
                None => return,
            };
            cell.objs.swap_remove(p);
        }

        let sing_cell = ll == ur;
        for id in cell_range(ll, ur) {
            let cell = storage.cell_mut_unchecked(id);
            cell.objs.push((handle, sing_cell))
        }
    }

    /// Removes an object from the grid.
    pub fn remove(&mut self, handle: AABBGridHandle) -> Option<O> {
        let st = self.objects.remove(handle)?;

        let storage = &mut self.storage;
        cells_apply(storage, &st.aabb, |cell, _| {
            for i in 0..cell.objs.len() {
                if cell.objs[i].0 == handle {
                    cell.objs.swap_remove(i);
                    return;
                }
            }
        });

        Some(st.obj)
    }

    /// Iterate over all handles
    pub fn handles(&self) -> impl Iterator<Item = AABBGridHandle> + '_ {
        self.objects.keys()
    }

    /// Iterate over all objects
    pub fn objects(&self) -> impl Iterator<Item = &O> + '_ {
        self.objects.values().map(|x| &x.obj)
    }

    /// Returns a reference to the associated object and its position, using the handle.
    pub fn get(&self, id: AABBGridHandle) -> Option<&StoreObject<O, AB>> {
        self.objects.get(id)
    }

    /// Returns a mutable reference to the associated object and its position, using the handle.
    pub fn get_mut(&mut self, id: AABBGridHandle) -> Option<&mut StoreObject<O, AB>> {
        self.objects.get_mut(id)
    }

    /// The underlying storage
    pub fn storage(&self) -> &SparseStorage<AABBGridCell> {
        &self.storage
    }

    /// Queries for objects intersecting a given AABB.
    pub fn query(&self, aabb: AB) -> impl Iterator<Item = (AABBGridHandle, &AB, &O)> + '_ {
        self.query_broad(aabb).filter_map(move |h| {
            // Safety: All objects in the cells are guaranteed to be valid.
            let obj = unsafe { self.objects.get_unchecked(h) };
            if aabb.intersects(&obj.aabb) {
                Some((h, &obj.aabb, &obj.obj))
            } else {
                None
            }
        })
    }

    /// Queries for all objects in the cells intersecting the given AABB
    pub fn query_broad(&self, bbox: AB) -> impl Iterator<Item = AABBGridHandle> + '_ {
        let storage = &self.storage;

        let ll_id = storage.cell_id(bbox.ll());
        let ur_id = storage.cell_id(bbox.ur());

        let iter = cell_range(ll_id, ur_id)
            .flat_map(move |id| storage.cell(id))
            .flat_map(|x| x.objs.iter().copied());

        if ll_id == ur_id {
            QueryIter::Simple(iter)
        } else {
            QueryIter::Dedup(
                fnv::FnvHashSet::with_hasher(fnv::FnvBuildHasher::default()),
                iter,
            )
        }
    }

    /// Queries for objects intersecting a given AABB.
    /// Uses a visitor for slightly better performance.
    pub fn query_visitor(&self, aabb: AB, mut visitor: impl FnMut(AABBGridHandle, &AB, &O)) {
        self.query_broad_visitor(aabb, move |h| {
            // Safety: All objects in the cells are guaranteed to be valid.
            let obj = unsafe { self.objects.get_unchecked(h) };
            if aabb.intersects(&obj.aabb) {
                visitor(h, &obj.aabb, &obj.obj)
            }
        })
    }

    /// Queries for all objects in the cells intersecting the given AABB
    /// Uses a visitor for slightly better performance.
    pub fn query_broad_visitor(&self, bbox: AB, mut visitor: impl FnMut(AABBGridHandle)) {
        let storage = &self.storage;

        let ll_id = storage.cell_id(bbox.ll());
        let ur_id = storage.cell_id(bbox.ur());

        if ll_id == ur_id {
            let cell = storage.cell(ll_id).unwrap();
            for (h, _) in cell.objs.iter() {
                visitor(*h);
            }
            return;
        }

        let mut dedup = fnv::FnvHashSet::with_hasher(fnv::FnvBuildHasher::default());

        for celly in ll_id.1..=ur_id.1 {
            for cellx in ll_id.0..=ur_id.0 {
                let cell = match storage.cell((cellx, celly)) {
                    Some(x) => x,
                    None => continue,
                };

                for (h, sing_cell) in cell.objs.iter() {
                    if *sing_cell {
                        visitor(*h);
                        continue;
                    }
                    if dedup.insert(*h) {
                        visitor(*h);
                    }
                }
            }
        }
    }

    /// Returns the number of objects currently available
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Checks if the grid contains objects or not
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

fn cells_apply<AB: AABB>(
    storage: &mut SparseStorage<AABBGridCell>,
    bbox: &AB,
    f: impl Fn(&mut AABBGridCell, bool),
) {
    let ll = storage.cell_mut(bbox.ll()).0;
    let ur = storage.cell_mut(bbox.ur()).0;
    for id in cell_range(ll, ur) {
        f(storage.cell_mut_unchecked(id), ll == ur)
    }
}

enum QueryIter<T: Iterator<Item = (AABBGridHandle, bool)>> {
    Simple(T),
    Dedup(fnv::FnvHashSet<AABBGridHandle>, T),
}

impl<T: Iterator<Item = (AABBGridHandle, bool)>> Iterator for QueryIter<T> {
    type Item = AABBGridHandle;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            QueryIter::Simple(x) => x.next().map(|(x, _)| x),
            QueryIter::Dedup(seen, x) => {
                for (v, sing_cell) in x {
                    if sing_cell {
                        return Some(v);
                    }
                    if seen.insert(v) {
                        return Some(v);
                    }
                }
                None
            }
        }
    }
}

use crate::grid::{GridHandle, GridObjects, ObjectState};
use retain_mut::RetainMut;

pub type CellObject = (GridHandle, mint::Point2<f32>);

/// A single cell of the grid, can be empty
#[derive(Default, Clone)]
pub struct GridCell {
    pub objs: Vec<CellObject>,
    pub dirty: bool,
}

impl GridCell {
    pub fn maintain<T: Copy, U: Copy + Eq>(
        &mut self,
        objects: &mut GridObjects<T, U>,
        to_relocate: &mut Vec<CellObject>,
    ) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        self.objs.retain_mut(|(obj_id, obj_pos)| {
            let store_obj = &mut objects[*obj_id];

            match store_obj.state {
                ObjectState::NewPos => {
                    store_obj.state = ObjectState::Unchanged;
                    *obj_pos = store_obj.pos;
                    true
                }
                ObjectState::Relocate => {
                    store_obj.state = ObjectState::Unchanged;
                    to_relocate.push((*obj_id, store_obj.pos));
                    false
                }
                ObjectState::Removed => {
                    objects.remove(*obj_id);
                    false
                }
                ObjectState::Unchanged => true,
            }
        });
    }
}

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
        self_id: U,
        objects: &mut GridObjects<T, U>,
        to_relocate: &mut Vec<(U, CellObject)>,
    ) {
        self.dirty = false;
        self.objs.retain_mut(|(obj_id, obj_pos)| {
            let store_obj = &mut objects[*obj_id];
            match store_obj.state {
                ObjectState::NewPos => {
                    store_obj.state = ObjectState::Unchanged;
                    *obj_pos = store_obj.pos;
                    let relocate = store_obj.cell_id != self_id;
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
    }
}

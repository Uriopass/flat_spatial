//!
//! `flat_spatial` is a crate dedicated to spatial partitioning structures that are not based on trees
//! (which are recursive) but on simple flat structures such as grids.
//!
//! `Grid` partitions the space using cells of user defined width.
//! `AABBGrid` partitions the space using cells too, but stores Axis-Aligned Bounding Boxes.
//!
//! Check `Grid` and `AABBGrid` docs for more information.
//!

pub mod cell;
pub mod grid;
pub mod aabbgrid;
pub mod storage;

pub use grid::Grid;
pub use aabbgrid::AABBGrid;

pub trait Vec2: From<[f32; 2]> + Copy {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
}

pub trait AABB: Copy {
    type V2: Vec2;

    fn ll(&self) -> Self::V2;
    fn ur(&self) -> Self::V2;

    #[inline]
    fn intersects(&self, b: &Self) -> bool {
        let ll = self.ll();
        let ur = self.ur();

        let bll = b.ll();
        let bur = b.ur();

        let x = f32::abs((ll.x() + ur.x()) - (bll.x() + bur.x()))
            <= (ur.x() - ll.x() + bur.x() - bll.x());
        let y = f32::abs((ll.y() + ur.y()) - (bll.y() + bur.y()))
            <= (ur.y() - ll.y() + bur.y() - bll.y());

        x & y
    }
}

impl Vec2 for [f32; 2] {
    #[inline]
    fn x(&self) -> f32 {
        unsafe { *self.get_unchecked(0) }
    }

    #[inline]
    fn y(&self) -> f32 {
        unsafe { *self.get_unchecked(1) }
    }
}

#[cfg(feature = "euclid")]
mod euclid_impl {
    use super::Vec2;
    use super::AABB;
    use euclid::{Point2D, Vector2D};

    impl<U> Vec2 for Point2D<f32, U> {
        fn x(&self) -> f32 {
            self.x
        }
        fn y(&self) -> f32 {
            self.y
        }
    }

    impl<U> Vec2 for Vector2D<f32, U> {
        fn x(&self) -> f32 {
            self.x
        }
        fn y(&self) -> f32 {
            self.y
        }
    }

    impl<U> AABB for euclid::Rect<f32, U> {
        type V2 = Point2D<f32, U>;
        fn ll(&self) -> Self::V2 {
            self.origin
        }
        fn ur(&self) -> Self::V2 {
            self.origin + self.size
        }
    }
}

#[cfg(feature = "parry2d")]
mod parry2d_impl {
    use super::Vec2;
    use super::AABB as AABBTrait;
    use parry2d::bounding_volume::AABB;
    use parry2d::math::{Point, Vector};

    impl Vec2 for Point<f32> {
        fn x(&self) -> f32 {
            self.x
        }
        fn y(&self) -> f32 {
            self.y
        }
    }

    impl Vec2 for Vector<f32> {
        fn x(&self) -> f32 {
            self.x
        }
        fn y(&self) -> f32 {
            self.y
        }
    }

    impl AABBTrait for AABB {
        type V2 = Point<f32>;
        fn ll(&self) -> Self::V2 {
            self.mins
        }
        fn ur(&self) -> Self::V2 {
            self.maxs
        }
    }
}

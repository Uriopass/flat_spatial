use mint::Point2;

mod aabb;
mod circle;
mod segment;

pub use aabb::*;
pub use circle::*;
pub use segment::*;

fn dot(a: Point2<f32>, b: Point2<f32>) -> f32 {
    a.x * b.x + a.y * b.y
}

pub trait Intersect<T: Shape> {
    fn intersects(&self, shape: T) -> bool;
}

pub trait Shape: Copy + Intersect<AABB> {
    fn bbox(&self) -> AABB;
}

impl Shape for [f32; 2] {
    fn bbox(&self) -> AABB {
        AABB {
            ll: (*self).into(),
            ur: (*self).into(),
        }
    }
}

impl Intersect<AABB> for [f32; 2] {
    fn intersects(&self, aabb: AABB) -> bool {
        aabb.contains((*self).into())
    }
}

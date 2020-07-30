pub use super::*;
use mint::Point2;

#[derive(Clone, Copy)]
pub struct AABB {
    /// Lower left of the AABB
    pub ll: Point2<f32>,
    /// Upper right of the AABB
    pub ur: Point2<f32>,
}

impl AABB {
    pub fn new(p1: Point2<f32>, p2: Point2<f32>) -> Self {
        AABB {
            ll: Point2 {
                x: p1.x.min(p2.x),
                y: p1.y.min(p2.y),
            },
            ur: Point2 {
                x: p1.x.max(p2.x),
                y: p1.y.max(p2.y),
            },
        }
    }

    pub fn contains(&self, p: Point2<f32>) -> bool {
        p.x >= self.ll.x && p.y >= self.ll.y && p.x <= self.ur.x && p.y <= self.ur.y
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment> {
        let ul = Point2 {
            x: self.ll.x,
            y: self.ur.y,
        };
        let lr = Point2 {
            x: self.ur.x,
            y: self.ll.y,
        };
        let ll = self.ll;
        let ur = self.ur;

        std::iter::once(Segment::new(ll, lr))
            .chain(std::iter::once(Segment::new(lr, ur)))
            .chain(std::iter::once(Segment::new(ur, ul)))
            .chain(std::iter::once(Segment::new(ul, ll)))
    }
}

impl Shape for AABB {
    fn bbox(&self) -> AABB {
        *self
    }
}

impl Intersect<AABB> for AABB {
    fn intersects(&self, b: AABB) -> bool {
        let a = self;
        let x =
            f32::abs((a.ll.x + a.ur.x) - (b.ll.x + b.ur.x)) <= (a.ur.x - a.ll.x + b.ur.x - b.ll.x);
        let y =
            f32::abs((a.ll.y + a.ur.y) - (b.ll.y + b.ur.y)) <= (a.ur.y - a.ll.y + b.ur.y - b.ll.y);

        x && y
    }
}

impl Intersect<Circle> for AABB {
    fn intersects(&self, shape: Circle) -> bool {
        shape.intersects(*self)
    }
}

impl Intersect<Segment> for AABB {
    fn intersects(&self, shape: Segment) -> bool {
        shape.intersects(*self)
    }
}

impl Intersect<[f32; 2]> for AABB {
    fn intersects(&self, p: [f32; 2]) -> bool {
        self.contains(p.into())
    }
}

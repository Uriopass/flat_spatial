pub use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub src: Point2<f32>,
    pub dst: Point2<f32>,
}

impl Segment {
    pub fn new(src: Point2<f32>, dst: Point2<f32>) -> Self {
        Self { src, dst }
    }

    pub fn project(&self, p: Point2<f32>) -> Point2<f32> {
        let diff = Point2 {
            x: self.dst.x - self.src.x,
            y: self.dst.y - self.src.y,
        };
        let diff2 = Point2 {
            x: p.x - self.src.x,
            y: p.y - self.src.y,
        };
        let diff3 = Point2 {
            x: p.x - self.dst.x,
            y: p.y - self.dst.y,
        };

        let proj1 = dot(diff2, diff);
        let proj2 = -dot(diff3, diff);

        if proj1 <= 0.0 {
            self.src
        } else if proj2 <= 0.0 {
            self.dst
        } else {
            let lol = proj1 / dot(diff, diff);
            Point2 {
                x: self.src.x + diff.x * lol,
                y: self.src.y + diff.y * lol,
            }
        }
    }
}

impl Shape for Segment {
    fn bbox(&self) -> AABB {
        AABB::new(self.src, self.dst)
    }
}

impl Intersect<AABB> for Segment {
    fn intersects(&self, aabb: AABB) -> bool {
        aabb.contains(self.src)
            || aabb.contains(self.dst)
            || aabb.segments().any(|s| s.intersects(*self))
    }
}

fn ccw(a: Point2<f32>, b: Point2<f32>, c: Point2<f32>) -> bool {
    (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x)
}

impl Intersect<Segment> for Segment {
    fn intersects(&self, s: Segment) -> bool {
        ccw(self.src, s.src, s.dst) != ccw(self.dst, s.src, s.dst)
            && ccw(self.src, self.dst, s.src) != ccw(self.src, self.dst, s.dst)
    }
}

impl Intersect<Circle> for Segment {
    fn intersects(&self, c: Circle) -> bool {
        c.intersects(*self)
    }
}

impl Intersect<[f32; 2]> for Segment {
    fn intersects(&self, _p: [f32; 2]) -> bool {
        false
    }
}

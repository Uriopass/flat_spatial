use crate::storage::Storage;
use mint::Point2;

fn dot(a: Point2<f32>, b: Point2<f32>) -> f32 {
    a.x * b.x + a.y * b.y
}

pub trait Shape {
    type RangeIter;

    fn bbox(&self) -> AABB;
    fn intersects(&self, aabb: AABB) -> bool {
        self.bbox().intersects(aabb)
    }
}

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
        self.clone()
    }

    fn intersects(&self, b: AABB) -> bool {
        let a = self;
        let x =
            f32::abs((a.ll.x + a.ur.x) - (b.ll.x + b.ur.x)) <= (a.ur.x - a.ll.x + b.ur.x - b.ll.x);
        let y =
            f32::abs((a.ll.y + a.ur.y) - (b.ll.y + b.ur.y)) <= (a.ur.y - a.ll.y + b.ur.y - b.ll.y);

        return x && y;
    }
}

#[derive(Clone, Copy)]
pub struct Circle {
    pub center: Point2<f32>,
    pub radius: f32,
}

impl Shape for Circle {
    fn bbox(&self) -> AABB {
        AABB {
            ll: Point2 {
                x: self.center.x - self.radius,
                y: self.center.y - self.radius,
            },
            ur: Point2 {
                x: self.center.x + self.radius,
                y: self.center.y + self.radius,
            },
        }
    }

    fn intersects(&self, b: AABB) -> bool {
        b.contains(self.center)
            || b.segments().any(|x| {
                let p = x.project(self.center);
                dot(p, p) < self.radius * self.radius
            })
    }
}

#[derive(Clone, Copy)]
pub struct Segment {
    pub src: Point2<f32>,
    pub dst: Point2<f32>,
}

impl Segment {
    pub fn new(src: Point2<f32>, dst: Point2<f32>) -> Self {
        Self { src, dst }
    }

    pub fn project(&self, p: Point2<f32>) -> Point2<f32> {
        let test = self.dst - self.src;

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

    fn intersects(&self, aabb: AABB) -> bool {
        aabb.contains(self.src)
            || aabb.contains(self.dst)
            || aabb.segments().any(|s| s.intersects(aabb))
    }
}

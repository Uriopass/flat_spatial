pub use super::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
}

impl Intersect<AABB> for Circle {
    fn intersects(&self, b: AABB) -> bool {
        let r1 = AABB {
            ll: Point2 {
                x: b.ll.x - self.radius,
                y: b.ll.y,
            },
            ur: Point2 {
                x: b.ur.x + self.radius,
                y: b.ur.y,
            },
        };

        let r2 = AABB {
            ll: Point2 {
                x: b.ll.x,
                y: b.ll.y - self.radius,
            },
            ur: Point2 {
                x: b.ur.x,
                y: b.ur.y + self.radius,
            },
        };

        if r1.contains(self.center) || r2.contains(self.center) {
            return true;
        }
        let r3 = AABB {
            ll: Point2 {
                x: b.ll.x - self.radius,
                y: b.ll.y - self.radius,
            },
            ur: Point2 {
                x: b.ur.x + self.radius,
                y: b.ur.y + self.radius,
            },
        };

        if !r3.contains(self.center) {
            return false;
        }

        let ul = Point2 {
            x: b.ll.x - self.center.x,
            y: b.ur.y - self.center.y,
        };
        let lr = Point2 {
            x: b.ur.x - self.center.x,
            y: b.ll.y - self.center.y,
        };
        let ll = Point2 {
            x: b.ll.x - self.center.x,
            y: b.ll.y - self.center.y,
        };
        let ur = Point2 {
            x: b.ur.x - self.center.x,
            y: b.ur.y - self.center.y,
        };

        let r2 = self.radius.powi(2);
        dot(ul, ul) < r2 || dot(lr, lr) < r2 || dot(ll, ll) < r2 || dot(ur, ur) < r2
    }
}

impl Intersect<Circle> for Circle {
    fn intersects(&self, c: Circle) -> bool {
        let v = Point2 {
            x: self.center.x - c.center.x,
            y: self.center.y - c.center.y,
        };

        dot(v, v) < (self.radius + c.radius).powi(2)
    }
}

impl Intersect<Segment> for Circle {
    fn intersects(&self, s: Segment) -> bool {
        let p = s.project(self.center);
        let diff = Point2 {
            x: p.x - self.center.x,
            y: p.y - self.center.y,
        };

        dot(diff, diff) < self.radius.powi(2)
    }
}

impl Intersect<[f32; 2]> for Circle {
    fn intersects(&self, p: [f32; 2]) -> bool {
        let diff = Point2 {
            x: self.center.x - p[0],
            y: self.center.y - p[1],
        };

        dot(diff, diff) < self.radius.powi(2)
    }
}

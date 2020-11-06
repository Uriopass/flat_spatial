pub use super::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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
        let x = self.center.x.min(b.ur.x).max(b.ll.x) - self.center.x;
        let y = self.center.y.min(b.ur.y).max(b.ll.y) - self.center.y;
        x * x + y * y < self.radius * self.radius
    }
}

impl Intersect<Circle> for Circle {
    fn intersects(&self, c: Circle) -> bool {
        let v = Point2 {
            x: self.center.x - c.center.x,
            y: self.center.y - c.center.y,
        };

        dot(v, v) < (self.radius + c.radius) * (self.radius + c.radius)
    }
}

impl Intersect<Segment> for Circle {
    fn intersects(&self, s: Segment) -> bool {
        let p = s.project(self.center);
        let diff = Point2 {
            x: p.x - self.center.x,
            y: p.y - self.center.y,
        };

        dot(diff, diff) < self.radius * self.radius
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

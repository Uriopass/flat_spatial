use mint::Point2;

pub trait Shape {
    fn bbox(&self) -> AABB;
    fn intersects(&self, aabb: AABB) -> bool {
        self.bbox().intersects(aabb)
    }
}

#[derive(Clone)]
pub struct AABB {
    /// Lower left of the AABB
    pub ll: Point2<f32>,
    /// Upper right of the AABB
    pub ur: Point2<f32>,
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
        let a = self;
        let x =
            f32::abs((a.ll.x + a.ur.x) - (b.ll.x + b.ur.x)) <= (a.ur.x - a.ll.x + b.ur.x - b.ll.x);
        let y =
            f32::abs((a.ll.y + a.ur.y) - (b.ll.y + b.ur.y)) <= (a.ur.y - a.ll.y + b.ur.y - b.ll.y);

        return x && y;
    }
}

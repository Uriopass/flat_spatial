use euclid::{Rect, Size2D};
use flat_spatial::AABBGrid;
use std::time::{Duration, Instant};

type Data = [f32; 5];

const SIZE: f32 = 500.0;
const QUERY_POP: i32 = 100_000;

const QUERY_N: u64 = 1_000_000;
const CELL_SIZE: i32 = 10;

fn query_setup_shape(s: i32) -> AABBGrid<Data, Rect<f32, ()>> {
    let mut grid = AABBGrid::new(s);
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        grid.insert(
            Rect::new((SIZE * r[0], SIZE * r[1]).into(), Size2D::zero()),
            [r[2], r[3], r[4], r[5], r[6]],
        );
    });
    grid
}

#[inline(never)]
fn black_box<T>(_x: T) {
    ()
}

#[inline(never)]
fn query_5_shapegrid(g: &AABBGrid<Data, Rect<f32, ()>>, iter: u64) -> Duration {
    let grid = g.clone();
    let start = Instant::now();

    for _ in 0..iter {
        let pos = [rand::random::<f32>() * SIZE, rand::random::<f32>() * SIZE];
        for x in grid.query(Rect::new(pos.into(), Size2D::new(5.0, 5.0))) {
            black_box(x);
        }
    }

    start.elapsed()
}

fn main() {
    let sg5 = query_setup_shape(CELL_SIZE);
    let t = query_5_shapegrid(&sg5, QUERY_N);
    println!("query 5 shape simple 1M: {}ms", t.as_millis());
}

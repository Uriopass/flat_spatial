use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flat_spatial::densegrid::DenseGrid;
use rstar::{RTree, RTreeObject};
use std::time::{Duration, Instant};

// Density: 0.4 pop/m^2
const QUERY_POP: i32 = 100_000;
const SIZE: f32 = 500.0;

// Data to store along the objects. Here about 20 bytes
type Data = [f32; 5];

#[derive(Clone)]
struct Rtreedata {
    pos: [f32; 2],
    data: Data,
}

impl RTreeObject for Rtreedata {
    type Envelope = rstar::AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        rstar::AABB::from_point(self.pos)
    }
}

fn query_setup(s: i32) -> DenseGrid<Data> {
    let mut grid: DenseGrid<[f32; 5]> = DenseGrid::new_centered(s, SIZE as i32 / s);
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

#[inline(never)]
fn query_5_densegrid(g: &DenseGrid<[f32; 5]>, iter: u64) -> Duration {
    let grid = g.clone();
    let start = Instant::now();

    for _ in 0..iter {
        let pos = [rand::random::<f32>() * SIZE, rand::random::<f32>() * SIZE];
        for x in grid.query_around(pos, 5.0) {
            black_box(x);
        }
    }

    start.elapsed()
}

fn query_5_kdtree(tree: &rstar::RTree<Rtreedata>, iter: u64) -> Duration {
    let tree = tree.clone();
    let start = Instant::now();
    for _ in 0..iter {
        let pos: [f32; 2] = rand::random();
        for x in tree.locate_in_envelope(&rstar::AABB::from_corners(
            [pos[0] * SIZE - 5.0, pos[1] * SIZE + 5.0],
            [pos[0] * SIZE + 5.0, pos[1] * SIZE + 5.0],
        )) {
            black_box((x, x.data));
        }
    }
    start.elapsed()
}

fn query(c: &mut Criterion) {
    let mut c = c.benchmark_group("Query");
    let g5 = query_setup(5);
    let g10 = query_setup(10);
    let g20 = query_setup(20);

    let mut tree = RTree::new();
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        tree.insert(Rtreedata {
            pos: [SIZE * r[0], SIZE * r[1]].into(),
            data: [r[2], r[3], r[4], r[5], r[6]],
        });
    });

    c.bench_function("query denseGrid05", |b| {
        b.iter_custom(|iter| query_5_densegrid(&g5, iter))
    });
    c.bench_function("query denseGrid10", |b| {
        b.iter_custom(|iter| query_5_densegrid(&g10, iter))
    });
    c.bench_function("query denseGrid20", |b| {
        b.iter_custom(|iter| query_5_densegrid(&g20, iter))
    });
    c.bench_function("query kdtree", |b| {
        b.iter_custom(|iter| query_5_kdtree(&tree, black_box(iter)))
    });
    c.finish()
}

fn maintain_densegrid(s: i32, iter: u64) -> Duration {
    let mut grid: DenseGrid<[f32; 5]> = DenseGrid::new_centered(s, SIZE as i32 / s);
    let mut handles = Vec::with_capacity(iter as usize);
    for _ in 0..iter {
        let r = rand::random::<[f32; 7]>();
        handles.push(grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]));
    }
    let start = Instant::now();

    for h in handles {
        grid.set_position(h, [rand::random(), rand::random()]);
    }
    grid.maintain();

    start.elapsed()
}

fn maintain_kdtree_seq(iter: u64) -> Duration {
    let start = Instant::now();
    let mut tree = RTree::new();
    for _ in 0..iter {
        let r = rand::random::<[f32; 7]>();
        tree.insert(Rtreedata {
            pos: [SIZE * r[0], SIZE * r[1]].into(),
            data: [r[2], r[3], r[4], r[5], r[6]],
        });
    }
    start.elapsed()
}

fn maintain_kdtree_bulk(iter: u64) -> Duration {
    let start = Instant::now();

    let v = (0..iter)
        .map(|_| {
            let r = rand::random::<[f32; 7]>();
            Rtreedata {
                pos: [SIZE * r[0], SIZE * r[1]].into(),
                data: [r[2], r[3], r[4], r[5], r[6]],
            }
        })
        .collect();
    let tree = RTree::bulk_load(v);
    black_box(tree);
    start.elapsed()
}

fn maintain(c: &mut Criterion) {
    let mut g = c.benchmark_group("Maintain");
    g.bench_function("maintain densegrid5", |b| {
        b.iter_custom(|iter| maintain_densegrid(black_box(5), iter))
    });
    g.bench_function("maintain densegrid10", |b| {
        b.iter_custom(|iter| maintain_densegrid(black_box(5), iter))
    });
    g.bench_function("maintain densegrid20", |b| {
        b.iter_custom(|iter| maintain_densegrid(black_box(5), iter))
    });
    g.bench_function("maintain kdtree", |b| {
        b.iter_custom(|iter| maintain_kdtree_seq(black_box(iter)))
    });
    g.bench_function("maintain kdtree bulk load", |b| {
        b.iter_custom(|iter| maintain_kdtree_bulk(black_box(iter)))
    });
    g.finish()
}

fn simple_bench() {
    let g5 = query_setup(10);
    let t = query_5_densegrid(&g5, 1000000);
    println!("query 5 dense simple 1M: {}ms", t.as_millis());

    let mut tree = RTree::new();
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        tree.insert(Rtreedata {
            pos: [SIZE * r[0], SIZE * r[1]].into(),
            data: [r[2], r[3], r[4], r[5], r[6]],
        });
    });

    let t = query_5_kdtree(&tree, 1000000);
    println!("query 5 kdtree simple 1M: {}ms", t.as_millis());

    let t = maintain_densegrid(10, 10_000_000);
    println!("maintain dense simple 10M: {}ms", t.as_millis());

    let t = maintain_kdtree_bulk(10_000_000);
    println!("maintain kdtree bulk simple 10M: {}ms", t.as_millis());
}

criterion_group!(benches,maintain, query);

criterion_main!(simple_bench, benches);

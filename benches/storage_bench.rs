use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flat_spatial::cell::ShapeGridCell;
use flat_spatial::storage::{DenseStorage, SparseStorage};
use flat_spatial::{DenseGrid, ShapeGrid, SparseGrid};
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
    let mut grid: DenseGrid<Data> = DenseGrid::new(s);
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

fn query_setup_sparse(s: i32) -> SparseGrid<Data> {
    let mut grid: SparseGrid<Data> = SparseGrid::new(s);
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

fn query_setup_shape(s: i32) -> ShapeGrid<Data, [f32; 2]> {
    let mut grid = ShapeGrid::new(s);
    (0..QUERY_POP).for_each(|_| {
        let r = rand::random::<[f32; 7]>();
        grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

#[inline(never)]
fn query_5_densegrid(g: &DenseGrid<Data>, iter: u64) -> Duration {
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

#[inline(never)]
fn query_5_sparsegrid(g: &SparseGrid<Data>, iter: u64) -> Duration {
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

#[inline(never)]
fn query_5_shapegrid(g: &ShapeGrid<Data, [f32; 2]>, iter: u64) -> Duration {
    let grid = g.clone();
    let start = Instant::now();

    for _ in 0..iter {
        let pos = [rand::random::<f32>() * SIZE, rand::random::<f32>() * SIZE];
        for x in grid.query(flat_spatial::shape::AABB {
            ll: [pos[0] - 5.0, pos[1] - 5.0].into(),
            ur: [pos[0] + 5.0, pos[1] + 5.0].into(),
        }) {
            black_box(x);
        }
    }

    start.elapsed()
}

#[inline(never)]
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

    let sg5 = query_setup_sparse(5);
    let sg10 = query_setup_sparse(10);
    let sg20 = query_setup_sparse(20);

    let sh5 = query_setup_shape(5);
    let sh10 = query_setup_shape(10);
    let sh20 = query_setup_shape(20);

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

    c.bench_function("query sparseGrid05", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg5, iter))
    });
    c.bench_function("query sparseGrid10", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg10, iter))
    });
    c.bench_function("query sparseGrid20", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg20, iter))
    });

    c.bench_function("query shapeGrid05", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh5, iter))
    });
    c.bench_function("query shapeGrid10", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh10, iter))
    });
    c.bench_function("query shapeGrid20", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh20, iter))
    });

    c.bench_function("query kdtree", |b| {
        b.iter_custom(|iter| query_5_kdtree(&tree, black_box(iter)))
    });
    c.finish()
}

fn maintain_densegrid(s: i32, iter: u64) -> Duration {
    let mut grid: DenseGrid<Data> =
        DenseGrid::with_storage(DenseStorage::new_centered(s, SIZE as i32 / s));
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

fn maintain_sparsegrid(s: i32, iter: u64) -> Duration {
    let mut grid: SparseGrid<Data> = SparseGrid::new(s);
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

fn maintain_shapegrid(s: i32, iter: u64) -> Duration {
    let mut grid: ShapeGrid<Data, [f32; 2]> = ShapeGrid::new(s);
    let mut handles = Vec::with_capacity(iter as usize);
    for _ in 0..iter {
        let r = rand::random::<[f32; 7]>();
        handles.push(grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]));
    }
    let start = Instant::now();

    for h in handles {
        grid.set_shape(h, [rand::random(), rand::random()]);
    }

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
        b.iter_custom(|iter| maintain_densegrid(black_box(10), iter))
    });
    g.bench_function("maintain densegrid20", |b| {
        b.iter_custom(|iter| maintain_densegrid(black_box(20), iter))
    });
    g.bench_function("maintain sparsegrid5", |b| {
        b.iter_custom(|iter| maintain_sparsegrid(black_box(5), iter))
    });
    g.bench_function("maintain sparsegrid10", |b| {
        b.iter_custom(|iter| maintain_sparsegrid(black_box(10), iter))
    });
    g.bench_function("maintain sparsegrid20", |b| {
        b.iter_custom(|iter| maintain_sparsegrid(black_box(20), iter))
    });
    g.bench_function("maintain shapegrid5", |b| {
        b.iter_custom(|iter| maintain_shapegrid(black_box(5), iter))
    });
    g.bench_function("maintain shapegrid10", |b| {
        b.iter_custom(|iter| maintain_shapegrid(black_box(10), iter))
    });
    g.bench_function("maintain shapegrid20", |b| {
        b.iter_custom(|iter| maintain_shapegrid(black_box(20), iter))
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
    let g5 = query_setup(5);
    let t = query_5_densegrid(&g5, 1_000_000);
    println!("query 5 dense simple 1M: {}ms", t.as_millis());

    let sg5 = query_setup_sparse(10);
    let t = query_5_sparsegrid(&sg5, 1_000_000);
    println!("query 5 sparse simple 1M: {}ms", t.as_millis());

    let sg5 = query_setup_shape(10);
    let t = query_5_shapegrid(&sg5, 1_000_000);
    println!("query 5 shape simple 1M: {}ms", t.as_millis());

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

    const M: u64 = 5_000_000;
    let t = maintain_densegrid(10, M);
    println!("maintain dense simple 5M: {}ms", t.as_millis());

    let t = maintain_sparsegrid(10, M);
    println!("maintain sparse simple 5M: {}ms", t.as_millis());

    let t = maintain_shapegrid(10, M);
    println!("maintain shape simple 5M: {}ms", t.as_millis());

    let t = maintain_kdtree_bulk(M);
    println!("maintain kdtree bulk simple 5M: {}ms", t.as_millis());
}

criterion_group!(benches, query, maintain);

criterion_main!(simple_bench, benches);

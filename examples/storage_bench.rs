use criterion::{black_box, Criterion};
use flat_spatial::{AABBGrid, Grid};
use rand::{Rng, SeedableRng};
use rstar::{RTree, RTreeObject};
use std::time::{Duration, Instant};

const QUERY_POP: i32 = 500_000;
const SIZE: f32 = 1000.0;
const DENSITY: f32 = (QUERY_POP as f32) / (SIZE * SIZE);

// Data to store along the objects. Here about 20 bytes
type Data = [f32; 5];

#[derive(Copy, Clone)]
struct AABB {
    ll: [f32; 2],
    ur: [f32; 2],
}

impl flat_spatial::AABB for AABB {
    type V2 = [f32; 2];

    fn ll(&self) -> Self::V2 {
        self.ll
    }

    fn ur(&self) -> Self::V2 {
        self.ur
    }
    /*
    #[inline]
    fn intersects(&self, b: &Self) -> bool {
        let x = f32::abs((self.ll[0] + self.ur[0]) - (b.ll[0] + b.ur[0]))
            <= (self.ur[0] - self.ll[0] + b.ur[0] - b.ll[0]);
        let y = f32::abs((self.ll[1] + self.ur[1]) - (b.ll[1] + b.ur[1]))
            <= (self.ur[1] - self.ll[1] + b.ur[1] - b.ll[1]);

        x & y
    }*/
}

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

fn query_setup_sparse(s: i32) -> Grid<Data, [f32; 2]> {
    let mut grid: Grid<Data, [f32; 2]> = Grid::new(s);
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    (0..QUERY_POP).for_each(|_| {
        let r = rng.gen::<[f32; 7]>();
        grid.insert([SIZE * r[0], SIZE * r[1]], [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

fn query_setup_shape(s: i32) -> AABBGrid<Data, AABB> {
    let mut grid = AABBGrid::new(s);
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    (0..QUERY_POP).for_each(|_| {
        let r = rng.gen::<[f32; 7]>();
        let p = [SIZE * r[0], SIZE * r[1]];
        grid.insert(AABB { ll: p, ur: p }, [r[2], r[3], r[4], r[5], r[6]]);
    });
    grid
}

#[inline(never)]
fn query_5_sparsegrid(g: &Grid<Data, [f32; 2]>, iter: u64) -> (Duration, u64) {
    let grid = g.clone();
    let start = Instant::now();

    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut hashres = 0;

    for _ in 0..iter {
        let pos = [rng.gen::<f32>() * SIZE, rng.gen::<f32>() * SIZE];
        grid.query_aabb_visitor(
            [pos[0] - 5.0, pos[1] - 5.0],
            [pos[0] + 5.0, pos[1] + 5.0],
            |x| {
                hashres += 1;
                black_box(x);
            },
        );
    }

    (start.elapsed(), hashres)
}

#[inline(never)]
fn query_5_shapegrid(g: &AABBGrid<Data, AABB>, iter: u64) -> (Duration, u64) {
    let grid = g.clone();
    let start = Instant::now();

    let mut rng = rand::rngs::StdRng::seed_from_u64(0);

    let mut hashres = 0;

    for _ in 0..iter {
        let pos = [rng.gen::<f32>() * SIZE, rng.gen::<f32>() * SIZE];
        grid.query_visitor(
            AABB {
                ll: [pos[0] - 5.0, pos[1] - 5.0],
                ur: [pos[0] + 5.0, pos[1] + 5.0],
            },
            |x, _, _| {
                hashres += 1;
                black_box(x);
            },
        )
    }

    (start.elapsed(), hashres)
}

#[inline(never)]
fn query_5_kdtree(tree: &rstar::RTree<Rtreedata>, iter: u64) -> (Duration, u64) {
    let start = Instant::now();

    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut hashres = 0;

    for _ in 0..iter {
        let pos = [rng.gen::<f32>() * SIZE, rng.gen::<f32>() * SIZE];
        for x in tree.locate_in_envelope(&rstar::AABB::from_corners(
            [pos[0] - 5.0, pos[1] - 5.0],
            [pos[0] + 5.0, pos[1] + 5.0],
        )) {
            hashres += 1;
            black_box((x, x.data));
        }
    }
    (start.elapsed(), hashres)
}

#[inline(never)]
fn query_5_kdbush(tree: &kdbush::KDBush, iter: u64) -> (Duration, u64) {
    let start = Instant::now();

    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut hashres = 0;

    for _ in 0..iter {
        let pos = [rng.gen::<f32>() * SIZE, rng.gen::<f32>() * SIZE];
        tree.range(
            (pos[0] - 5.0) as f64,
            (pos[1] - 5.0) as f64,
            (pos[0] + 5.0) as f64,
            (pos[1] + 5.0) as f64,
            |x| {
                hashres += 1;
                black_box(x);
            },
        )
    }
    (start.elapsed(), hashres)
}

fn query(c: &mut Criterion) {
    let mut c = c.benchmark_group("Query");
    let sg5 = query_setup_sparse(5);
    let sg10 = query_setup_sparse(10);
    let sg20 = query_setup_sparse(20);

    let sh5 = query_setup_shape(5);
    let sh10 = query_setup_shape(10);
    let sh20 = query_setup_shape(20);

    let mut tree = RTree::new();

    let mut rng = rand::rngs::StdRng::seed_from_u64(0);

    (0..QUERY_POP).for_each(|_| {
        let r = rng.gen::<[f32; 7]>();
        tree.insert(Rtreedata {
            pos: [SIZE * r[0], SIZE * r[1]].into(),
            data: [r[2], r[3], r[4], r[5], r[6]],
        });
    });

    c.bench_function("query sparseGrid05", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg5, iter).0)
    });
    c.bench_function("query sparseGrid10", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg10, iter).0)
    });
    c.bench_function("query sparseGrid20", |b| {
        b.iter_custom(|iter| query_5_sparsegrid(&sg20, iter).0)
    });

    c.bench_function("query shapeGrid05", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh5, iter).0)
    });
    c.bench_function("query shapeGrid10", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh10, iter).0)
    });
    c.bench_function("query shapeGrid20", |b| {
        b.iter_custom(|iter| query_5_shapegrid(&sh20, iter).0)
    });

    c.bench_function("query kdtree", |b| {
        b.iter_custom(|iter| query_5_kdtree(&tree, black_box(iter)).0)
    });
    c.finish()
}

fn maintain_sparsegrid(s: i32, iter: u64) -> Duration {
    let mut grid: Grid<Data, [f32; 2]> = Grid::new(s);
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
    let start = Instant::now();

    let mut grid: AABBGrid<Data, AABB> = AABBGrid::new(s);
    let mut handles = Vec::with_capacity(iter as usize);
    for _ in 0..iter {
        let r = rand::random::<[f32; 7]>();
        let p = [SIZE * r[0], SIZE * r[1]];
        handles.push(grid.insert(AABB { ll: p, ur: p }, [r[2], r[3], r[4], r[5], r[6]]));
    }

    black_box(grid);
    /*
    for h in handles {
        let p = [rand::random(), rand::random()];
        grid.set_aabb(h, AABB { ll: p, ur: p });
    }*/

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

fn maintain_kdbush_bulk(iter: u64) -> Duration {
    let start = Instant::now();

    let v: Vec<(f64, f64)> = (0..iter)
        .map(|_| {
            let r = rand::random::<[f32; 7]>();
            ((SIZE * r[0]) as f64, (SIZE * r[1]) as f64)
        })
        .collect();
    let tree = kdbush::KDBush::create(v, 10);
    black_box(tree);
    start.elapsed()
}

fn maintain(c: &mut Criterion) {
    let mut g = c.benchmark_group("Maintain");
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

fn main() {
    println!("Density is {}", DENSITY);
    let mut pos: Vec<(f64, f64)> = vec![];

    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    (0..QUERY_POP).for_each(|_| {
        let r = rng.gen::<[f32; 7]>();
        pos.push(((SIZE * r[0]) as f64, (SIZE * r[1]) as f64));
    });

    let tree = kdbush::KDBush::create(pos, 10);

    let (t, hash) = query_5_kdbush(&tree, 300_000);
    println!(
        "query 5 kdbush simple 1M: {}ms hash:{}",
        t.as_millis(),
        hash
    );

    let mut tree = RTree::new();
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    (0..QUERY_POP).for_each(|_| {
        let r = rng.gen::<[f32; 7]>();
        tree.insert(Rtreedata {
            pos: [SIZE * r[0], SIZE * r[1]].into(),
            data: [r[2], r[3], r[4], r[5], r[6]],
        });
    });
    let (t, hash) = query_5_kdtree(&tree, 300_000);
    println!(
        "query 5 kdtree simple 1M: {}ms hash:{}",
        t.as_millis(),
        hash
    );

    let sg5 = query_setup_sparse(10);
    let (t, hash) = query_5_sparsegrid(&sg5, 300_000);
    println!(
        "query 5 sparse simple 1M: {}ms hash:{}",
        t.as_millis(),
        hash
    );

    let sg5 = query_setup_shape(10);
    let (t, hash) = query_5_shapegrid(&sg5, 300_000);
    println!("query 5 shape simple 1M: {}ms hash:{}", t.as_millis(), hash);

    const M: u64 = 5_000_000;
    let t = maintain_sparsegrid(10, M);
    println!("maintain sparse simple 5M: {}ms", t.as_millis());

    let t = maintain_shapegrid(10, M);
    println!("maintain shape simple 5M: {}ms", t.as_millis());

    let t = maintain_kdtree_bulk(M);
    println!("maintain kdtree bulk simple 5M: {}ms", t.as_millis());

    let t = maintain_kdbush_bulk(M);
    println!("maintain kdbush bulk simple 5M: {}ms", t.as_millis());
}

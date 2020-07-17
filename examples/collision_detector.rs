use flat_spatial::Grid;

// A structure has to be copy in order to be in a dense grid, because of slotmap's requirements.
// This is subject to change
#[derive(Copy, Clone)]
struct Car {
    direction: [f32; 2],
}

fn main() {
    // Creates the grid with cell size 10
    let mut g: Grid<Car> = Grid::new(10);

    // create objects in the range x: [-50..50], y: [-50..50]
    for _ in 0..100 {
        let pos = [
            100.0 * rand::random::<f32>() - 50.0,
            100.0 * rand::random::<f32>() - 50.0,
        ];
        let magn = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
        g.insert(
            pos,
            Car {
                direction: [-pos[0] / magn, -pos[1] / magn],
            },
        );
    }

    for _ in 0..50 {
        update_loop(&mut g);
    }
}

fn update_loop(g: &mut Grid<Car>) {
    println!("{} cars left", g.len());

    let handles: Vec<_> = g.handles().collect();
    // Handle collisions (remove on collide)
    for &h in &handles {
        let (pos, _car) = g.get(h).unwrap();

        let mut collided = false;
        for (other_h, other_pos) in g.query_around(pos, 8.0) {
            if other_h == h {
                continue;
            }
            if (other_pos.x - pos.x).powi(2) + (other_pos.y - pos.y).powi(2) < 2.0 * 2.0 {
                collided = true;
                break;
            }
        }

        if collided {
            g.remove(h);
        }
    }

    // Update positions
    for h in handles {
        let (pos, car) = g.get(h).unwrap();
        let dir = car.direction;
        g.set_position(h, [pos.x + dir[0], pos.y + dir[1]])
    }

    // Handle position updates and removals
    g.maintain();
}

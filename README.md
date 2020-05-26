# flat_spatial

flat_spatial is a crate dedicated to dynamic spatial partitioning structures that are not based on trees
(which are recursive) but on simple flat structures such as a grid/hashmap of cells.  
Using grids or other flat structures makes for very fast updates (constant time) and
even fast queries, provded they are adapted to the structure.

At the moment, only the dense grid is implemented.

## DenseGrid

![](https://i.imgur.com/2rkQbxB.png)

The idea of a dense grid is to have an array of cells which store the positions 
of the inserted objects.  
Performing queries is as simple as looking up which cells are affected and returning 
their associated objects.  
Since it's so simple, the dense grid supports dynamic capabilities such as position update
or object removal based on handles (using `slotmap`).

It is recommended to have queries roughly the same size as the cell size.

Here is an example of a densegrid usage for a (very simple) collision detector:

### Example

```Rust
use flat_spatial::DenseGrid;

// A structure has to be copy in order to be in a dense grid, because of slotmap's requirements. 
// This is subject to change
#[derive(Copy, Clone)]
struct Car {
    direction: [f32; 2],
}

fn main() {
    // Creates the grid with cell size 10
    let mut g: DenseGrid<Car> = DenseGrid::new(10);
    
    // create objects in the range x: [-50..50], y: [-50..50]
    for _ in 0..100 {
        let pos = [100.0 * rand::random::<f32>() - 50.0, 100.0 * rand::random::<f32>() - 50.0];
        let magn = (pos[0].powi(2) + pos[1].powi(2)).sqrt();
        g.insert(
            pos,
            Car {
                direction: [-pos[0] / magn, -pos[1] / magn],
            },
        );
    }

    loop {
        update_loop(&mut g);
    }
}

fn update_loop(g: &mut DenseGrid<Car>) {
    let handles: Vec<_> = g.handles().collect();
    // Handle collisions (remove on collide)
    for &h in &handles {
        let (pos, _car) = g.get(h).unwrap();

        let mut collided = false;
        for (other_h, other_pos) in g.query_around(pos, 8.0) {
            if other_h == &h {
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
    for &h in &handles {
        let (pos, car) = g.get(h).unwrap();
        g.set_position(h, [pos.x + car.direction[0], pos.y + car.direction[1]])
    }

    // Handle position updates and removals
    g.maintain();
}
```
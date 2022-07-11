# flat_spatial

[![Build Status](https://github.com/Uriopass/flat_spatial/workflows/Rust/badge.svg?branch=master)](https://github.com/Uriopass/flat_spatial/actions)
[![Crates.io](https://img.shields.io/crates/v/flat_spatial.svg)](https://crates.io/crates/flat_spatial)
[![Docs.rs](https://docs.rs/flat_spatial/badge.svg)](https://docs.rs/flat_spatial)

flat_spatial is a crate dedicated to dynamic spatial partitioning structures that are not based on trees
(which are recursive) but on simple flat structures such as a grid of cells.  
Using grids or other flat structures makes for very fast updates (constant time) and
even fast queries, provided the cell size is adapted to the problem.

MSRV: 1.60

## Grid

![](https://i.imgur.com/2rkQbxB.png)

The idea of a grid is to have a HashMap of cells which store the positions 
of the inserted objects.  
Performing queries is as simple as looking up which cells are affected and returning 
their associated objects.  
Since it's so simple, the grid supports dynamic capabilities such as position update
or object removal based on handles (using `slotmap`).
The position updates are lazy for better performance, so maintain() needs to be called to update the grid.

It is recommended to have queries roughly the same size as the cell size.

## AABBGrid

The aabbgrid is like a grid but it stores Axis-Aligned Bounding Boxes (AABB) instead of positions.
This implemented as a HashMap of cells which store the AABB that touches it.
For each cell an AABB touches, it is added to the cell. Try to keep the aabb sizes as small as possible.

Adding/updating/removing isn't lazy, no need to call maintain.

### Example

Here is a very basic example of the grid:

```Rust
fn main() {
    use flat_spatial::Grid;
    
    let mut g: Grid<(), [f32; 2]> = Grid::new(10);
    let a = g.insert([3.0, 3.0], ());
    let _b = g.insert([12.0, -8.0], ());
    
    let around: Vec<_> = g.query_around([2.0, 2.0], 5.0)
                          .map(|(id, _pos)| id)
                          .collect();
     
    assert_eq!(vec![a], around);
}
```

//!
//! flat_spatial is a crate dedicated to spatial partitioning structures that are not based on trees
//! (which are recursive) but on simple flat structures such as grids.
//!
//! Both DenseGrid and SparseGrid partition the space using cells of user defined width.
//! DenseGrid uses a Vec of cells and SparseGrid a HashMap (so cells are lazily allocated).
//!

pub mod cell;
pub mod grid;
pub mod storage;

pub use grid::Grid;
use storage::DenseStorage;
use storage::SparseStorage;

pub type DenseGrid<O> = Grid<O, DenseStorage>;
pub type SparseGrid<O> = Grid<O, SparseStorage>;

mod shape;

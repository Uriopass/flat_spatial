//!
//! flat_spatial is a crate dedicated to spatial partitioning structures that are not based on trees
//! (which are recursive) but on simple flat structures such as grids.
//!
//! Both DenseGrid and SparseGrid partition the space using cells of user defined width. $
//! DenseGrid uses a Vec of cells and SparseGrid a HashMap (so cells are lazily allocated).
//!

pub mod densegrid;
pub mod sparsegrid;

pub use densegrid::DenseGrid;
pub use sparsegrid::SparseGrid;

//!
//! flat_spatial is a crate dedicated to spatial partitioning structures that are not based on trees
//! (which are recursive) but on simple flat structures such as grids.
//!
//! At the moment, only the dense grid is implemented, which partitions the space using cells
//! of user defined width. However the sparse grid structure is planned, where cells are allocated
//! lazily.
//!

pub mod densegrid;
pub use densegrid::DenseGrid;

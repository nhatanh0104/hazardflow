//! Mesh.

#![allow(unused)] // Added for assignment.

use super::tile::*;
use super::*;

/// Mesh row data. It consists of `MESH_ROWS` tile row data.
pub type MeshRowData = [TileRowData; MESH_ROWS];

/// Mesh column data. It consists of `MESH_COLS` tile column data.
pub type MeshColData = [TileColData; MESH_COLS];

/// Applies a 1-cycle delay register to the row-side egress interface of a tile.
///
/// This helper function is used with the `array_map!` macro, as the macro currently does not accept closures as arguments.
fn reg_fwd_tile_row(i: Valid<PeRowData>) -> Valid<PeRowData> {
    i.reg_fwd_always()
}

/// Applies a 1-cycle delay register to the column-side egress interface of a tile.
///
/// This helper function is used with the `array_map!` macro, as the macro currently does not accept closures as arguments.
fn reg_fwd_tile_col((i1, i2): (Valid<PeColData>, Valid<PeColControl>)) -> (Valid<PeColData>, Valid<PeColControl>) {
    (i1.reg_fwd_always(), i2.reg_fwd_always())
}

/// A tile with a 1-cycle delay register attached to each egress interface.
///
/// This is used as a component within the Mesh.
pub fn tile_with_reg(in_left: TileRowData, in_top: TileColData) -> (TileRowData, TileColData) {
    let (out_right, out_bottom) = tile(in_left, in_top);

    // NOTE: The `array_map!` macro currently does not accept closures as arguments, so we defined helper functions
    //       instead of inlining it.
    (array_map!(out_right, reg_fwd_tile_row), array_map!(out_bottom, reg_fwd_tile_col))
}

/// Mesh.
pub fn mesh(in_left: MeshRowData, in_top: MeshColData) -> (MeshRowData, MeshColData) {
    let arr = from_fn(flip(tile_with_reg));
    let row = flip(seq(arr));
    let tile = seq(from_fn(row));

    tile(in_left, in_top)
}

/// Mesh with default Gemmini configuration (16 x 16 Tiles).
#[synthesize]
pub fn mesh_default(in_left: MeshRowData, in_top: MeshColData) -> (MeshRowData, MeshColData) {
    mesh(in_left, in_top)
}

//! Transposer.

#![allow(unused)] // Added for assignment.

use super::*;

/// Indicates the direction of the Transposer PE.
#[derive(Debug, Default, Clone, Copy)]
enum Dir {
    /// Selects data from row side.
    #[default]
    Row,

    /// Selects data from column side.
    Col,
}

impl Dir {
    fn flip(self) -> Self {
        match self {
            Dir::Row => Dir::Col,
            Dir::Col => Dir::Row,
        }
    }
}

/// Transposer PE.
fn transposer_pe(
    in_row: Valid<S<INPUT_BITS>>,
    (in_col, in_dir): (Valid<S<INPUT_BITS>>, Valid<Dir>),
) -> (Valid<S<INPUT_BITS>>, (Valid<S<INPUT_BITS>>, Valid<Dir>)) {
    let (in_dir_fork, out_dir) = in_dir.lfork();
    let in_dir_mux = in_dir_fork.map(|p| {
        match p {
            Dir::Row => U::from(0),
            Dir::Col => U::from(1),
        }
    });
    let reg_out = [in_row, in_col].mux(in_dir_mux).reg_fwd_valid();
    let (out_row, out_col) = reg_out.lfork();
    (out_row, (out_col, out_dir))
}

/// Systolic array of Transposer PEs.
#[allow(clippy::type_complexity)]
fn transposer_pes<const DIM: usize>(
    in_row: [Valid<S<INPUT_BITS>>; DIM],
    in_col_with_dir: [(Valid<S<INPUT_BITS>>, Valid<Dir>); DIM],
) -> ([Valid<S<INPUT_BITS>>; DIM], [(Valid<S<INPUT_BITS>>, Valid<Dir>); DIM]) {
    let arr = from_fn(flip(transposer_pe));
    let row = flip(seq(arr));
    let tile = seq(from_fn(row));

    tile(in_row, in_col_with_dir)
}

/// Unzips the valid interfaces in the array.
fn unzip_tuple_arr<P1: Copy, P2: Copy, const N: usize>(i: [Valid<(P1, P2)>; N]) -> [(Valid<P1>, Valid<P2>); N] {
    // NOTE: The `array_map!` macro currently does not accept closures as arguments, so we explicitly used `Valid::<(P1, P2)>::unzip`
    //       instead of `move |i| i.unzip()`.
    array_map!(i, Valid::<(P1, P2)>::unzip)
}

/// Zips the valid interfaces in the array.
fn zip_tuple_arr<P1: Copy, P2: Copy, const N: usize>(i: [(Valid<P1>, Valid<P2>); N]) -> [Valid<(P1, P2)>; N] {
    // NOTE: The `array_map!` macro currently does not accept closures as arguments, so we explicitly used `JoinValidExt::join_valid`
    //       instead of `move |(i1, i2)| (i1, i2).join_valid()`.
    array_map!(i, JoinValidExt::join_valid)
}

/// Transposer.
pub fn transposer<const DIM: usize>(i: Valid<Array<S<INPUT_BITS>, DIM>>) -> Valid<Array<S<INPUT_BITS>, DIM>>
where
    [(); clog2(DIM)]:,
    [(); clog2(DIM) + 1]:,
{
    let reg = i.fsm_map((0, Dir::default()), |p, s| {
        let new_state = if s.0 != DIM - 1 {
            (s.0 + 1, s.1)
        } else {
            match s.1 {
                Dir::Row => (0, Dir::Col),
                Dir::Col => (0, Dir::Row),
            }
        };

        ((p, s.1), new_state)
    });

    let (data, dir) = reg.unzip();
    
    // Pre-process
    let (in_row, in_col) = data.lfork();
    let (dir_pes, dir_mux) = dir.lfork();

    let in_row_pes = in_row.map(|p| p.reverse()).unzip();
    let in_col_with_dir =(in_col, dir_pes)
        .join_valid()
        .map(|p| p.0.map(|e| {
            let new_dir = p.1;
            (e, new_dir)
        }))
        .map(|p| p.reverse())
        .unzip();
    let in_col_with_dir_pes = unzip_tuple_arr(in_col_with_dir);
    
    // Feed into systolic array
    let (out_row_pes, out_col_with_dir_pes) = transposer_pes(in_row_pes, in_col_with_dir_pes);

    // Post-process
    let out_row = out_row_pes.join_valid().map(|p| p.reverse());
    let out_col_with_dir = zip_tuple_arr(out_col_with_dir_pes);
    let out_col = out_col_with_dir
        .join_valid()
        .map(|p| p.map(|e| e.0))
        .map(|p| p.reverse());

    // Mux
    let in_dir_mux = dir_mux.map(|p| {
        match p {
            Dir::Row => U::from(0),
            Dir::Col => U::from(1),
        }
    });
    
    let out = [out_row, out_col].mux(in_dir_mux);

    out
}

/// Transposer with default Gemmini configuration (16 x 16 Transposer PEs).
#[synthesize]
pub fn transposer_default(in_row: Valid<Array<S<INPUT_BITS>, 16>>) -> Valid<Array<S<INPUT_BITS>, 16>> {
    transposer::<16>(in_row)
}

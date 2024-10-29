//! Interface.

use hazardflow_macro::magic;

use super::*;
use crate::prelude::*;

/// Interface trait.
#[must_use]
pub trait Interface: Sized {
    /// Forward signal.
    type Fwd: Copy;

    /// Backward signal.
    type Bwd: Copy;

    /// A generic FSM combinator.
    ///
    /// We assume that the function `f` is combinational logic. The returned egress payload and ingress resolver are
    /// immediately propagated, and the state is updated to the returned next state from the next cycle.
    ///
    /// # Safety
    ///
    /// When using this combinator, you need to guarantee that it satisfies the specification of the interface's
    /// protocol.
    ///
    /// In particular, for a hazard interface [`I<H, D>`], you must follow the specification described in the "Safety"
    /// section of [`I::fsm`].
    ///
    /// # How it is compiled?
    ///
    /// In the HazardFlow compiler, the fsm function is not actually executed (which would lead to a panic).
    ///
    /// Instead, the HazardFlow compiler captures the [High-level IR](https://rustc-dev-guide.rust-lang.org/hir.html)
    /// generated by the Rust compiler and extracts information about the ingress/egress/state types (`Self`, `E`, `S`)
    /// and the arguments (`init_state`, `f`) of the fsm function.
    ///
    /// Using this information, the HazardFlow compiler generates the corresponding Verilog code.
    ///
    /// # Type parameters
    ///
    /// - `Self`: The ingress interface type.
    /// - `E`: The egress interface type.
    /// - `S`: The state type.
    ///
    /// # Parameters
    ///
    /// - `self`: The ingress interface.
    /// - `init_state`: The initial state.
    ///     - Whenever `rst` signal is turned on, the state will be initialized to this value.
    ///     - For example, set `init_state` as `None` for a state with an `Option<_>` type.
    /// - `f`: Output calculation and state transition logic. If `let (ep, ir, s_next) = f(ip, er, s)`,
    ///     - `ip`: The ingress payload.
    ///     - `er`: The egress resolver.
    ///     - `s`: The current state.
    ///     - `ep`: The egress payload.
    ///     - `ir`: The ingress resolver.
    ///     - `s_next`: The next state.
    #[magic(interface::fsm)]
    unsafe fn fsm<E: Interface, S: Copy>(
        self,
        _init_state: S,
        _f: impl Fn(Self::Fwd, E::Bwd, S) -> (E::Fwd, Self::Bwd, S),
    ) -> E {
        compiler_magic!()
    }

    /// Combines the module to the given interface and returns the egress interface.
    fn comb<E: Interface>(self, m: impl FnOnce(Self) -> E) -> E {
        m(self)
    }
}

impl Interface for () {
    type Bwd = ();
    type Fwd = ();
}

impl<If: Interface, const N: usize> Interface for [If; N] {
    type Bwd = Array<If::Bwd, N>;
    type Fwd = Array<If::Fwd, N>;
}

macro_rules! impl_interface_tuple {
    ($($a:ident)+) => {
        impl<$($a: Interface,)+> Interface for ($($a,)+) {
            type Fwd = ($($a::Fwd,)+);
            type Bwd = ($($a::Bwd,)+);
        }
    }
}

impl_interface_tuple! { If1 }
impl_interface_tuple! { If1 If2 }
impl_interface_tuple! { If1 If2 If3 }
impl_interface_tuple! { If1 If2 If3 If4 }
impl_interface_tuple! { If1 If2 If3 If4 If5 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 If8 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 If8 If9 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 If8 If9 If10 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 If8 If9 If10 If11 }
impl_interface_tuple! { If1 If2 If3 If4 If5 If6 If7 If8 If9 If10 If11 If12 }

#[allow(missing_docs)]
#[macro_export]
macro_rules! array_map {
    ($s: ident, $f: expr) => {{
        let ms = from_fn(|i, j| ($f(i), j));
        let seq = seq(ms);
        let (e, _) = seq($s, ());
        e
    }};
}

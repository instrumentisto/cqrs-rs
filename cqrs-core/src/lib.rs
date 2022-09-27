//! Core types for the [CQRS]/[ES] aggregate system.
//!
//! [CQRS]: https://martinfowler.com/bliki/CQRS.html
//! [ES]: https://martinfowler.com/eaaDev/EventSourcing.html

#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use
)]
#![warn(
    missing_docs,
    missing_copy_implementations,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results
)]
//#![warn(unreachable_pub)]

mod aggregate;
mod command;

mod event;
//mod into;

use std::pin::Pin;

use futures::Stream;

#[doc(inline)]
pub use self::{aggregate::*, command::*, event::*};

/// Helper alias for pin-boxed `?Send` [`Stream`] which yields [`Result`]s.
pub type LocalBoxTryStream<'a, I, E> = Pin<Box<dyn Stream<Item = Result<I, E>> + 'a>>;

/// Macro used to concat slices at compile time.
#[macro_export]
macro_rules! const_concat_slices {
    ($ty:ty, $a:expr) => {$a};
    ($ty:ty, $a:expr, $b:expr $(,)*) => {{
        const A: &[$ty] = $a;
        const B: &[$ty] = $b;
        const __LEN: usize = A.len() + B.len();
        const __CONCATENATED: &[$ty; __LEN] = &{
            let mut out: [$ty; __LEN] = if __LEN == 0 {
                unsafe {
                    ::core::mem::transmute(
                        [0u8; ::core::mem::size_of::<$ty>() * __LEN],
                    )
                }
            } else if A.len() == 0 {
                [B[0]; { A.len() + B.len() }]
            } else {
                [A[0]; { A.len() + B.len() }]
            };
            let mut i = 0;
            while i < A.len() {
                out[i] = A[i];
                i += 1;
            }
            i = 0;
            while i < B.len() {
                out[i + A.len()] = B[i];
                i += 1;
            }
            out
        };

        __CONCATENATED
    }};
    ($ty:ty, $a:expr, $b:expr, $($c:expr),+ $(,)*) => {
        $crate::const_concat_slices!(
            $ty,
            $a,
            $crate::const_concat_slices!($ty, $b, $($c),+)
        )
    };
}

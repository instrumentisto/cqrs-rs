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

/// Concatenates slices at compile time.
#[macro_export]
macro_rules! const_concat_slices {
    ($a:expr) => {$a};
    ($a:expr, $b:expr $(,)*) => {
        $crate::private::slice_arr(
            &const {
                const __LEN: usize = 255;
                if $a.len() + $b.len() > __LEN {
                    panic!("concatenated slices exceed maximum length");
                }

                let mut out = [""; __LEN];
                let mut i = 0;
                while i < $a.len() {
                    out[i] = $a[i];
                    i += 1;
                }
                i = 0;
                while i < $b.len() {
                    out[i + $a.len()] = $b[i];
                    i += 1;
                }
                out
            },
            $a.len() + $b.len(),
        )
    };
    ($a:expr, $b:expr, $($c:expr),+ $(,)*) => {
        $crate::const_concat_slices!(
            $a,
            $crate::const_concat_slices!($b, $($c),+)
        )
    };
}

#[doc(hidden)]
pub mod private {
    /// Slices an array of strings at compile time.
    pub const fn slice_arr<const N: usize>(
        arr: &'static [&'static str; N],
        at: usize,
    ) -> &'static [&'static str] {
        #[allow(unsafe_code, reason = "macro internals")]
        unsafe { std::slice::from_raw_parts(arr.as_ptr(), at) }
    }
}

//! # Feature flags
//!
//! This crate exposes a number of features. These can be enabled or disabled as shown
//! [in Cargo's documentation](https://doc.rust-lang.org/cargo/reference/features.html). Features
//! are _disabled_ by default unless otherwise noted.
//!
//! Reliance on a given feature is always indicated alongside the item definition.
//!
//! - `std` (_enabled by default, implicitly enables `alloc`_)
//!
//!   This enables a number of features that depend on the standard library. [`Instant`] is the
//!   primary item that requires this feature, though some   others methods may rely on [`Instant`]
//!   internally.
//!
//! - `alloc` (_enabled by default via `std`_)
//!
//!   Enables a number of features that require the ability to dynamically allocate memory.
//!
//! - `macros`
//!
//!   Enables macros that provide compile-time verification of values and intuitive syntax.
//!
//! - `formatting` (_implicitly enables `std`_)
//!
//!   Enables formatting of most structs.
//!
//! - `parsing`
//!
//!   Enables parsing of most structs.
//!
//! - `local-offset` (_implicitly enables `std`_)
//!
//!   This feature enables a number of methods that allow obtaining the system's UTC offset.
//!
//! - `large-dates`
//!
//!   By default, only years within the ±9999 range (inclusive) are supported. If you need support
//!   for years outside this range, consider enabling this feature; the supported range will be
//!   increased to ±999,999.
//!
//!   Note that enabling this feature has some costs, as it means forgoing some optimizations.
//!   Ambiguities may be introduced when parsing that would not otherwise exist.
//!
//!   If you are using this feature, **please leave a comment**
//!   [on this discussion](https://github.com/time-rs/time/discussions/306) with your use case. If
//!   there is not sufficient demand for this feature, it will be dropped in a future release.
//!
//! - `serde`
//!
//!   Enables [serde](https://docs.rs/serde) support for all types.
//!
//! - `serde-human-readable` (_implicitly enables `serde`, `formatting`, and `parsing`_)
//!
//!   Allows serde representations to use a human-readable format. This is determined by the
//!   serializer, not the user. If this feature is not enabled or if the serializer requests a
//!   non-human-readable format, a format optimized for binary representation will be used.
//!
//! - `rand`
//!
//!   Enables [rand](https://docs.rs/rand) support for all types.
//!
//! - `quickcheck` (_implicitly enables `alloc`_)
//!
//!   Enables [quickcheck](https://docs.rs/quickcheck) support for all types except [`Instant`].
//!
//! One pseudo-feature flag that is only available to end users is the `unsound_local_offset` cfg.
//! As the name indicates, using the feature is unsound, and [may cause unexpected segmentation
//! faults](https://github.com/time-rs/time/issues/293). Unlike other flags, this is deliberately
//! only available to end users; this is to ensure that a user doesn't have unsound behavior without
//! knowing it. To enable this behavior, you must use `RUSTFLAGS="--cfg unsound_local_offset" cargo
//! build` or similar. Note: This flag is _not tested anywhere_, including in the regular test of
//! the powerset of all feature flags. Use at your own risk.

#![doc(html_playground_url = "https://play.rust-lang.org")]
#![cfg_attr(__time_03_docs, feature(doc_cfg))]
#![cfg_attr(
    __time_03_docs,
    deny(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    anonymous_parameters,
    clippy::all,
    const_err,
    illegal_floating_point_literal_pattern,
    late_bound_lifetime_arguments,
    path_statements,
    patterns_in_fns_without_body,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_extern_crates
)]
#![warn(
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::get_unwrap,
    clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    missing_copy_implementations,
    missing_debug_implementations,
    unused_qualifications,
    variant_size_differences
)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::enum_glob_use,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::redundant_pub_crate
)]
#![doc(html_favicon_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(test(attr(deny(warnings))))]

#[allow(unused_extern_crates)]
#[cfg(feature = "alloc")]
extern crate alloc;

// region: macros
/// Division of integers, rounding the resulting value towards negative infinity.
macro_rules! div_floor {
    ($a:expr, $b:expr) => {{
        let _a = $a;
        let _b = $b;

        let (_quotient, _remainder) = (_a / _b, _a % _b);

        if (_remainder > 0 && _b < 0) || (_remainder < 0 && _b > 0) {
            _quotient - 1
        } else {
            _quotient
        }
    }};
}

/// Euclidean remainder. Useful for `const` contexts.
macro_rules! rem_euclid {
    ($a:expr, $b:expr) => {{
        let _a = $a;
        let _b = $b;
        let r = _a % _b;
        if r < 0 {
            if _b < 0 { r - _b } else { r + _b }
        } else {
            r
        }
    }};
}

/// Cascade an out-of-bounds value.
macro_rules! cascade {
    (@discard_if_underscore(_); $($x:tt)*) => {};
    (@discard_if_underscore($i:ident); $($x:tt)*) => { $($x)* };
    (@discard_if_not_mut(mut); $($x:tt)*) => {};
    (@discard_if_not_mut(); $($x:tt)*) => { $($x)* };
    (@ordinal) => {};
    (@year) => {};

    // Cascade an out-of-bounds value from "from" to "to".
    ($(!$from_not_mut:ident)? $from:ident in $min:literal.. $max:literal => $to:tt) => {
        #[allow(unused_comparisons)]
        if $from >= $max {
            cascade!(@discard_if_not_mut($($from_not_mut)?); $from -= $max - $min);
            cascade!(@discard_if_underscore($to); $to += 1);
        } else if $from < $min {
            cascade!(@discard_if_not_mut($($from_not_mut)?); $from += $max - $min);
            cascade!(@discard_if_underscore($to); $to -= 1);
        }
    };

    // Special case the ordinal-to-year cascade, as it has different behavior.
    ($(!$ordinal_not_mut:ident)? $ordinal:ident => $(!$year_not_mut:ident)? $year:ident) => {
        // We need to actually capture the idents. Without this, macro hygiene causes errors.
        cascade!(@$ordinal);
        cascade!(@$year);
        if $ordinal > crate::util::days_in_year($year) {
            cascade!(@discard_if_not_mut($($year_not_mut)?); $year += 1);
            cascade!(@discard_if_not_mut($($ordinal_not_mut)?); $ordinal = 1);
        } else if $ordinal == 0 {
            cascade!(@discard_if_not_mut($($year_not_mut)?); $year -= 1);
            cascade!(@discard_if_not_mut($($ordinal_not_mut)?);
                $ordinal = crate::util::days_in_year($year)
            );
        }
    };
}

/// Returns `Err(error::ComponentRange)` if the value is not in range.
macro_rules! ensure_value_in_range {
    ($value:ident in $start:expr => $end:expr) => {
        #[allow(clippy::manual_range_contains)] // rust-lang/rust-clippy#6373
        #[allow(trivial_numeric_casts, unused_comparisons)]
        if $value < $start || $value > $end {
            return Err(crate::error::ComponentRange {
                name: stringify!($value),
                minimum: $start as _,
                maximum: $end as _,
                value: $value as _,
                conditional_range: false,
            });
        }
    };

    ($value:ident conditionally in $start:expr => $end:expr) => {
        #[allow(clippy::manual_range_contains)] // rust-lang/rust-clippy#6373
        #[allow(trivial_numeric_casts, unused_comparisons)]
        if $value < $start || $value > $end {
            return Err(crate::error::ComponentRange {
                name: stringify!($value),
                minimum: $start as _,
                maximum: $end as _,
                value: $value as _,
                conditional_range: true,
            });
        }
    };
}

/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but does not perform `.into()`. Because of this, it is
/// usable in `const` contexts.
macro_rules! const_try {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            Err(error) => return Err(error),
        }
    };
}

/// Try to unwrap an expression, returning if not possible.
///
/// This is similar to the `?` operator, but is usable in `const` contexts.
macro_rules! const_try_opt {
    ($e:expr) => {
        match $e {
            Some(value) => value,
            None => return None,
        }
    };
}
// endregion macros

/// The [`Date`] struct and its associated `impl`s.
mod date;
/// The [`Duration`] struct and its associated `impl`s.
mod duration;
/// Various error types returned by methods in the time crate.
pub mod error;
/// Extension traits.
pub mod ext;
#[cfg(any(feature = "formatting", feature = "parsing"))]
#[cfg_attr(
    __time_03_docs,
    doc(cfg(any(feature = "formatting", feature = "parsing")))
)]
pub mod format_description;
#[cfg(feature = "formatting")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "formatting")))]
mod formatting;
/// The [`Instant`] struct and its associated `impl`s.
#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
mod instant;
/// Macros to construct statically known values.
#[cfg(feature = "macros")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "macros")))]
pub mod macros;
/// The [`OffsetDateTime`] struct and its associated `impl`s.
mod offset_date_time;
#[cfg(feature = "parsing")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "parsing")))]
pub mod parsing;
/// The [`PrimitiveDateTime`] struct and its associated `impl`s.
mod primitive_date_time;
#[cfg(feature = "quickcheck")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "quickcheck")))]
mod quickcheck;
#[cfg(feature = "rand")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "rand")))]
mod rand;
#[cfg(feature = "serde")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "serde")))]
#[allow(missing_copy_implementations, missing_debug_implementations)]
pub mod serde;
#[cfg(test)]
mod tests;
/// The [`Time`] struct and its associated `impl`s.
mod time;
/// The [`UtcOffset`] struct and its associated `impl`s.
mod utc_offset;
pub mod util;
/// Days of the week.
mod weekday;

pub use crate::date::Date;
pub use crate::duration::Duration;
pub use crate::error::Error;
#[cfg(feature = "std")]
pub use crate::instant::Instant;
pub use crate::offset_date_time::OffsetDateTime;
pub use crate::primitive_date_time::PrimitiveDateTime;
pub use crate::time::Time;
pub use crate::utc_offset::UtcOffset;
pub use crate::weekday::Weekday;

/// An alias for [`std::result::Result`] with a generic error from the time crate.
pub type Result<T> = core::result::Result<T, Error>;

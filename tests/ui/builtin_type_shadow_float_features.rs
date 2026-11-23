//@revisions: neither only_f16 only_f128 both
//@[neither] check-pass
#![feature(allow_internal_unstable)]
#![allow(dead_code, internal_features, non_camel_case_types, unused_imports)]
#![deny(unfulfilled_lint_expectations)]
#![cfg_attr(any(only_f16, both), feature(f16))]
#![cfg_attr(any(only_f128, both), feature(f128))]
#![warn(clippy::builtin_type_shadow)]

// Libraries can supply these names on toolchains where the built-ins are unavailable.
struct Half;
use Half as f16;
//~[only_f16,both]^ builtin_type_shadow

#[allow(clippy::builtin_type_shadow)]
mod library {
    pub struct f128;
}

mod consumer {
    use super::library::*;
    //~[only_f128,both]^ builtin_type_shadow
}

fn generic<f16, f128>() {}
//~[only_f16,both]^ builtin_type_shadow
//~[only_f128,both]^^ builtin_type_shadow

// A macro may use the primitive even when its caller has not enabled the feature.
#[allow_internal_unstable(f16)]
macro_rules! half_precision {
    () => {
        mod builtin {
            const _: usize = core::mem::size_of::<f16>();
        }
        mod from_macro {
            #[expect(clippy::builtin_type_shadow)]
            type f16 = u16;
        }
    };
}
half_precision!();

fn main() {}

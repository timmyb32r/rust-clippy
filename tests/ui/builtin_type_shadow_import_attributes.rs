//@check-pass
#![deny(
    unfulfilled_lint_expectations,
    clippy::builtin_type_shadow,
    clippy::useless_attribute
)]

mod wide {
    #[allow(clippy::builtin_type_shadow)]
    pub use core::primitive::u16 as u8;
}

mod glob {
    #[expect(clippy::builtin_type_shadow)]
    use super::wide::*;
    const _: u8 = 300;
}

mod nested {
    #[expect(clippy::builtin_type_shadow)]
    use core::primitive::{bool as Flag, u16 as u8};
    const _: (Flag, u8) = (true, 300);
}

#[allow(clippy::builtin_type_shadow)]
extern crate core as i8;
const _: i8::primitive::u16 = 300;

mod expected_crate {
    #[expect(clippy::builtin_type_shadow)]
    extern crate core as u8;
    const _: u8::primitive::u16 = 300;
}

// The allowed lint applies to the import. The other attribute still has no effect.
#[expect(clippy::useless_attribute)]
mod neighboring_attribute {
    #[allow(clippy::builtin_type_shadow)]
    #[allow(clippy::almost_swapped)]
    use core::primitive::u16 as u8;

    const _: u8 = 300;
}

fn main() {}

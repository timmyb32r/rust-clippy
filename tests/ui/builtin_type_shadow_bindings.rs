#![allow(deprecated, nonstandard_style, unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]

mod imports {
    use core::primitive::u16 as u8;
    //~^ builtin_type_shadow
    pub const WIDE: u8 = 300;

    pub struct Packet;
    use self::{Packet as str, Packet as _};
    //~^ builtin_type_shadow
    extern crate core as isize;
    //~^ builtin_type_shadow
}

mod items {
    struct bool;
    //~^ builtin_type_shadow
    enum i8 {}
    //~^ builtin_type_shadow
    union u8 {
        //~^ builtin_type_shadow
        byte: core::primitive::u8,
    }
    mod i16 {}
    //~^ builtin_type_shadow
    trait u16 {}
    //~^ builtin_type_shadow
    type i32 = core::primitive::u32;
    //~^ builtin_type_shadow
}

mod aliases {
    type Byte = core::primitive::u8;
    type Alias = Byte;
    type u8 = Alias;
    use self::Alias as u16;
    //~^ builtin_type_shadow
    use core::primitive::{bool, char as Character};
    type char = Character;

    mod defaults {
        type u8<T = core::primitive::u8> = T;
        //~^ builtin_type_shadow
        fn inferred() {
            let n = u8::clone(&1);
            let _: i32 = n;
        }

        type u16<const N: usize = 1> = core::primitive::u16;
        type u32<const N: usize> = core::primitive::u32;
        //~^ builtin_type_shadow
    }

    mod lifetime {
        type u8<'a> = core::primitive::u8;
        //~^ builtin_type_shadow
        // A plain `byte: u8` field would now require a lifetime.
        struct Packet {
            byte: u8<'static>,
        }
    }

    mod projection {
        trait Bytes {
            type Item;
        }
        impl Bytes for () {
            type Item = core::primitive::u8;
        }
        type u8 = <() as Bytes>::Item;
    }
}

mod standard_modules {
    use core::str;
    use std::char;
    fn parse(s: &str) -> Result<char, str::Utf8Error> {
        Ok(s.chars().next().unwrap())
    }

    mod reexport {
        pub use std::str as text;
    }
    mod caller {
        use super::reexport::text as str;
    }
    mod renamed {
        use std::str as u8;
        //~^ builtin_type_shadow
    }
    mod legacy {
        #[warn(clippy::legacy_numeric_constants)]
        use std::u8;
        //~^ legacy_numeric_constants
    }
}

mod boundaries {
    use core::mem::drop as bool;
    fn u16() {}
    mod imported {
        #[allow(clippy::builtin_type_shadow)]
        pub use core::primitive::u16 as u8;
    }
    use imported::*;
    const VALUE: u8 = 300;
}
fn main() {}

mod attribute_levels {
    #![deny(clippy::useless_attribute, unfulfilled_lint_expectations)]

    #[expect(clippy::builtin_type_shadow)]
    use core::primitive::{i8, u16 as u8};
    #[expect(clippy::builtin_type_shadow)]
    extern crate core as u16;
}

mod unstable_names {
    type f16 = i16;
    fn generic<f128>() {}
}

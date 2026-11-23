//@aux-build:builtin_shadow_dep.rs
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

#[allow(clippy::builtin_type_shadow)]
mod bad {
    pub struct u8;
    pub struct i8;
}
#[allow(clippy::builtin_type_shadow)]
mod other {
    pub struct u8;
}

mod globs {
    mod same {
        use core::primitive::*;
        const SMALL: u8 = 1;
    }
    mod masked {
        use super::super::bad::*;
        //~^ builtin_type_shadow
        use core::primitive::u8; // i8 alone comes from the glob
    }
    mod ambiguous {
        use super::super::bad::*;
        //~^ builtin_type_shadow
        use super::super::other::*; // u8 is ambiguous and cannot be used
    }
    fn in_block() {
        use builtin_shadow_dep::types::*;
        //~^ builtin_type_shadow
        let _: u8 = 300;
    }
    fn unused_in_block() {
        use builtin_shadow_dep::types::*;
        //~^ builtin_type_shadow
    }
    fn masked_in_block() {
        use builtin_shadow_dep::types::*;
        use core::primitive::u8;
        let _: u8 = 1;
    }
    fn ambiguous_in_block() {
        use super::bad::*;
        //~^ builtin_type_shadow
        use super::other::*;
    }
    mod nested {
        use builtin_shadow_dep::types::{Byte as u16, *};
        //~^ builtin_type_shadow
        //~| builtin_type_shadow
    }
    mod variants {
        enum Format {
            u8,
        }
        use Format::*;
        //~^ builtin_type_shadow
    }
}

// An already ambiguous name, reexported through a third module, can make
// another block glob ambiguous even when the name is never used.
#[allow(ambiguous_glob_reexports)]
mod ambiguous_reexports {
    pub use super::bad::*;
    //~^ builtin_type_shadow
    pub use super::other::*;
}

fn transitive_ambiguity() {
    {
        use ambiguous_reexports::*;
        //~^ builtin_type_shadow
        use builtin_shadow_dep::types::*;
    }
    {
        use builtin_shadow_dep::ambiguous::*;
        use builtin_shadow_dep::types::*;
    }
}

fn main() {}

mod floating_point_modules {
    use core::f32;
    use std::f64;
    const PI32: f32 = f32::consts::PI;
    const PI64: f64 = f64::consts::PI;
}

mod renamed_standard_submodule {
    use std::f64::consts as f64;
    //~^ builtin_type_shadow
}

mod inferred_projection {
    trait Bytes {
        type Item;
    }
    impl<T> Bytes for T {
        type Item = core::primitive::u8;
    }

    // The projection normalizes to the primitive, but u8::MAX still needs T to be inferred.
    type u8<T = core::primitive::u8> = <T as Bytes>::Item;
    //~^ builtin_type_shadow
}

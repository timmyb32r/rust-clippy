//@rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
#![feature(decl_macro)]
#![allow(dead_code, non_upper_case_globals, unused_imports)]
#![warn(clippy::builtin_type_shadow)]

#[allow(clippy::builtin_type_shadow)]
extern crate self as u32;
pub const VALUE: usize = 3;

#[allow(clippy::builtin_type_shadow)]
mod qualified {
    pub use core::primitive::u16 as u8;
}

// Keep the braced imports in the macro bodies.
#[rustfmt::skip]
macro setup($defs:ident, $make_reader:ident, $byte:ident, $word:ident) {
    #[allow(clippy::builtin_type_shadow)]
    mod $defs {
        pub use crate as u32;
        pub use core::primitive::{u8 as $byte, u16 as u8, u32 as $word};
        pub const i8: usize = 2;
    }

    macro_rules! $make_reader {
        (qualified) => {
            use crate::qualified::{u8 as Byte};
            const _: [(); 2] = [(); size_of::<Byte>()];
        };
        ($reader:ident) => {
            pub macro $reader() {
                use {crate::qualified::{u8 as Byte}};
                const _: [(); 2] = [(); size_of::<Byte>()];
                // The generated module resolves u32 in the extern prelude.
                mod inner {
                    const _: [(); 3] = [(); u32::VALUE];
                }
            }
        };
        ($reader:ident, value) => {
            pub macro $reader() {
                use {{i8 as Bytes}};
                const _: [(); 2] = [(); Bytes];
            }
        };
    }
}
setup!(definitions, make_reader, u8, u32);

// A qualified import inside a macro uses no lexical binding named u8.
mod local_reader {
    use crate::definitions::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    make_reader!(qualified);
}

mod definition {
    use crate::definitions::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    const _: [(); 4] = [(); size_of::<u32>()];
    make_reader!(read);
}

// An empty group preserves lexical lookup. The inner i8 really comes from this glob.
mod lexical {
    pub use crate::definitions::*;
    //~^ builtin_type_shadow
    make_reader!(read, value);
}

mod invocation {
    crate::definition::read!();
    crate::lexical::read!();
}

fn main() {}

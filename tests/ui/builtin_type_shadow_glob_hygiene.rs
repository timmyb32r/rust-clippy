#![feature(decl_macro)]
#![warn(clippy::builtin_type_shadow)]
#![allow(dead_code, non_upper_case_globals, unused_imports)]

macro source($module:ident, $caller_name:ident) {
    pub mod $module {
        pub use core::primitive::{u16 as u8, u16 as $caller_name};
    }
}

#[allow(clippy::builtin_type_shadow)]
mod definitions {
    crate::source!(types, i8);
}

mod direct {
    use crate::definitions::types::*;
    //~^ builtin_type_shadow
    const _: [(); 1] = [(); core::mem::size_of::<u8>()];
    const _: i8 = 300;
}

fn direct_block() {
    use crate::definitions::types::*;
    //~^ builtin_type_shadow
    let _: [(); 1] = [(); core::mem::size_of::<u8>()];
    let _: i8 = 300;
}

mod reexport {
    pub use crate::definitions::types::*;
    //~^ builtin_type_shadow
}

mod transitive {
    use crate::reexport::*;
    //~^ builtin_type_shadow
    const _: [(); 1] = [(); core::mem::size_of::<u8>()];
    const _: i8 = 300;
}

fn transitive_block() {
    use crate::reexport::*;
    //~^ builtin_type_shadow
    let _: [(); 1] = [(); core::mem::size_of::<u8>()];
    let _: i8 = 300;
}

// The name starts in the caller's context, but the intermediate glob adds an opaque mark.
macro relay($module:ident) {
    pub mod $module {
        pub use crate::reexport::*;
        //~^ builtin_type_shadow
        const _: i8 = 300;
    }
}
relay!(opaque_reexport);

mod after_opaque_glob {
    use crate::opaque_reexport::*;
    const _: [(); 1] = [(); core::mem::size_of::<i8>()];

    fn block() {
        use crate::opaque_reexport::*;
        let _: [(); 1] = [(); core::mem::size_of::<i8>()];
    }
}

macro same_expansion($defs:ident) {
    #[allow(clippy::builtin_type_shadow)]
    mod $defs {
        pub use core::primitive::u16 as u8;
    }

    mod consumer {
        use super::$defs::*;
        //~^ builtin_type_shadow
        const _: u8 = 300;
    }

    fn in_block() {
        use $defs::*;
        //~^ builtin_type_shadow
        let _: u8 = 300;
    }
}
same_expansion!(same_defs);

// The call site and the generated reader resolve the same spelling differently.
macro make($defs:ident, $read:ident, $caller:ident) {
    #[allow(clippy::builtin_type_shadow)]
    mod $defs {
        pub use core::primitive::{u8 as $caller, u16 as u8};
        pub macro $caller() {
            17
        }
        pub const $caller: usize = 17;
    }
    macro_rules! $read {
        () => {
            const _: [(); 2] = [(); size_of::<u8>()];
        };
    }
}
make!(types, read, u8);
mod visible_to_macro {
    use crate::types::*;
    //~^ builtin_type_shadow
    const _: [(); 1] = [(); size_of::<u8>()];
    read!();
}
fn visible_to_macro_in_block() {
    use types::*;
    //~^ builtin_type_shadow
    const _: [(); 1] = [(); size_of::<u8>()];
    read!();
}
mod other_bindings {
    use crate::types::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    const _: usize = u8;
    const _: usize = u8!();

    fn inner() {
        use crate::types::*;
        //~^ builtin_type_shadow
        read!();
    }
}

// The opaque reader resolves its extra hygiene mark in the scope of its definition.
macro readers($defs:ident, $make_reader:ident, $caller:ident) {
    #[allow(clippy::builtin_type_shadow)]
    mod $defs {
        pub use core::primitive::{u8 as $caller, u16 as u8};
    }
    macro_rules! $make_reader {
        ($reader:ident) => {
            pub macro $reader() {
                const _: [(); 2] = [(); size_of::<u8>()];
            }
        };
        ($reader:ident, with_local_import) => {
            pub macro $reader() {
                const _: () = {
                    #[allow(clippy::builtin_type_shadow)]
                    use core::primitive::u16 as u8;
                    const _: [(); 2] = [(); size_of::<u8>()];
                };
            }
        };
    }
}
readers!(reader_types, make_reader, u8);

mod local_invocation {
    use crate::reader_types::*;
    //~^ builtin_type_shadow
    make_reader!(opaque_read);
    opaque_read!();
}

fn in_nested_block() {
    {
        use crate::reader_types::*;
        //~^ builtin_type_shadow
        make_reader!(opaque_read);
        opaque_read!();
    }
}

mod with_child {
    use crate::reader_types::*;
    //~^ builtin_type_shadow
    make_reader!(opaque_read);

    mod child {
        super::opaque_read!();
    }
}

mod sibling_definition {
    use crate::reader_types::*;
    //~^ builtin_type_shadow
    make_reader!(opaque_read);
}

mod unused_reader {
    use crate::reader_types::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    make_reader!(opaque_read);
}

mod sibling_invocation {
    use crate::reader_types::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    crate::sibling_definition::opaque_read!();
}

mod shadowed_in_reader {
    use crate::reader_types::*;
    const _: [(); 1] = [(); size_of::<u8>()];
    make_reader!(opaque_read, with_local_import);
    opaque_read!();
}

fn main() {}

//@aux-build:../../ui/auxiliary/builtin_shadow_dep.rs
//@aux-build:../../ui/auxiliary/proc_macros.rs
//@aux-build:../../ui/auxiliary/proc_macro_attr.rs
//@revisions: types values types_external values_external
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this generic shadows the built-in type `i8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u32`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[types] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types
//@[values] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
//@[types_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types_external
//@[values_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values_external
#![allow(nonstandard_style, unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]
#![deny(unfulfilled_lint_expectations)]

mod imported {
    use builtin_shadow_dep::types::*;
    //~^ builtin_type_shadow
    pub const WIDE: u8 = 300;
}

mod external {
    //~[types_external,values_external]v builtin_type_shadow
    proc_macros::external!(
        type u128 = i128;
    );
    proc_macros::with_span!(input type u32 = i32;);
    //~[types_external,values_external]^ builtin_type_shadow
    builtin_shadow_dep::declare_type!();
    //~[types_external,values_external]^ builtin_type_shadow
    proc_macros::with_span!(input fn generated<u16>(_: u16) {});
    //~[types_external,values_external]^ builtin_type_shadow
    // The identifier keeps its own input span, distinct from the generic brackets.
    proc_macros::with_span!(left fn distinct_spans<$u8>(_: $u8) {});
    //~[types_external,values_external]^ builtin_type_shadow
    builtin_shadow_dep::declare_generic!();
    //~[types_external,values_external]^ builtin_type_shadow
    builtin_shadow_dep::declare_value!();
    //~[values_external]^ builtin_type_shadow
}

mod forged_attribute {
    #[proc_macro_attr::forge_shadowed_names]
    //~[types_external,values_external]^ builtin_type_shadow
    //~[types_external,values_external]| builtin_type_shadow
    //~[types_external,values_external]| builtin_type_shadow
    fn original() {}
}

mod local {
    macro_rules! declare {
        () => {
            type i64 = u64;
            //~^ builtin_type_shadow
        };
    }
    declare!();
}

mod values {
    fn u8() {}
    //~[values,values_external]^ builtin_type_shadow

    const i16: usize = 0;
    //~[values,values_external]^ builtin_type_shadow
    static i32: usize = 0;
    //~[values,values_external]^ builtin_type_shadow

    fn parse<const u16: usize>(i8: bool) {
        //~[values,values_external]^ builtin_type_shadow
        //~[values,values_external]| builtin_type_shadow
        let (f32, _) = (1, 2);
        //~[values,values_external]^ builtin_type_shadow
        let _ = |char: char| char;
        //~[values,values_external]^ builtin_type_shadow
    }

    use self::u8 as u64;
    //~[values,values_external]^ builtin_type_shadow
    mod imported {
        use builtin_shadow_dep::values::*;
        //~[values,values_external]^ builtin_type_shadow
    }

    struct Reader {
        u8: usize,
    }
    impl Reader {
        fn u8(&self) -> usize {
            self.u8
        }
    }
}

mod included {
    include!("../../ui/auxiliary/builtin_shadow_included.inc");
}

#[proc_macro_attr::dummy]
fn passed_through<u32, r#u16>() {
    //~^ builtin_type_shadow
    //~| builtin_type_shadow
    let u8 = 1;
    //~[values,values_external]^ builtin_type_shadow
}

#[proc_macro_attr::dummy]
mod passed_through_globs {
    mod single {
        use builtin_shadow_dep::types::*;
        //~^ builtin_type_shadow
    }
    mod nested {
        use builtin_shadow_dep::types::{Byte as _, *};
        //~^ builtin_type_shadow
    }
}

fn parameter_levels<
    #[expect(clippy::builtin_type_shadow)] u8,
    #[allow(clippy::builtin_type_shadow)] u16,
    #[cfg_attr(any(values, values_external), expect(clippy::builtin_type_shadow))] const u32: usize,
    #[allow(clippy::builtin_type_shadow)] const u64: usize,
>() {
}

fn generated_locals() {
    // The containing function is local, but these patterns are generated. Even the
    // `let` token has the span of `original` in the macro input.
    proc_macros::with_span!(original let u8 = 1;);
    //~[values_external]^ builtin_type_shadow
    proc_macros::with_span!(original { let i16 = 2; });
    //~[values_external]^ builtin_type_shadow

    let r#u32 = 3;
    //~[values,values_external]^ builtin_type_shadow
    macro_rules! local {
        () => {
            let i8 = 4;
            //~[values,values_external]^ builtin_type_shadow
        };
    }
    local!();
}

// Known problem: the generated type parameter has both the spelling and the
// source location of the caller's function name.
#[cfg(types)]
mod matching_input_span {
    #[proc_macro_attr::forge_shadowed_names]
    fn u8() {}
    //~[types]^ builtin_type_shadow
}

mod generated_helpers {
    // The proc macro gives the helper declaration and its call the span of `original`.
    proc_macros::with_span!(original
        //~[types_external,values_external]^ builtin_type_shadow
        //~[types_external,values_external]| builtin_type_shadow
        macro_rules! helper { () => { type u8 = u16; fn choose<i8>() {} }; }
        helper!();
    );

    // The caller can also invoke an externally generated helper later.
    proc_macros::with_span!(original macro_rules! later { () => { type u32 = i32; }; });
    //~[types_external,values_external]^ builtin_type_shadow
    later!();

    fn local_binding() {
        proc_macros::with_span!(original
            //~[values_external]^ builtin_type_shadow
            macro_rules! declare { () => { let u8 = 1; }; }
            declare!();
        );
    }
}

mod external_include {
    builtin_shadow_dep::include_source!("../../ui/auxiliary/builtin_shadow_included.inc");
    //~[types_external,values_external]^ builtin_type_shadow
    //~[types_external,values_external]| builtin_type_shadow
    //~[values_external]| builtin_type_shadow
}

fn included_expressions() {
    let _direct = include!("../../ui/auxiliary/builtin_shadow_included_expr.inc");
    let _external = builtin_shadow_dep::include_source!("../../ui/auxiliary/builtin_shadow_included_expr.inc");
    //~[values_external]^ builtin_type_shadow

    macro_rules! local {
        () => {
            include!("../../ui/auxiliary/builtin_shadow_included_expr.inc")
        };
    }
    #[cfg_attr(any(values, values_external), expect(clippy::builtin_type_shadow))]
    let _local = local!();
}

fn includes_in_one_expression() {
    let _direct = (
        include!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
        include!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
    );
    let _external = (
        builtin_shadow_dep::include_source!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
        builtin_shadow_dep::include_source!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
    );
    let _mixed = (
        include!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
        builtin_shadow_dep::include_source!("../../ui/auxiliary/builtin_shadow_included_expr.inc"),
    );
}

fn main() {}

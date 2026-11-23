//@aux-build:../../ui/auxiliary/builtin_shadow_dep.rs
//@aux-build:../../ui/auxiliary/proc_macro_attr.rs
//@revisions: types values types_external values_external
//@[types] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types
//@[values] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
//@[types_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types_external
//@[values_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values_external
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u32`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i16`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this generic shadows the built-in type `i8`
//@error-in-other-file: this binding shadows the built-in type `u64`
//@error-in-other-file: this binding shadows the built-in type `u64`
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this generic shadows the built-in type `i8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
#![feature(proc_macro_hygiene, stmt_expr_attributes)]
#![allow(nonstandard_style, unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]

async fn desugared_operands() -> Option<()> {
    for _ in include!("includes/iterable.inc") {}
    for _ in builtin_shadow_dep::include_source!("includes/iterable.inc") {}
    //~[values_external]^ builtin_type_shadow

    include!("includes/optional.inc")?;
    builtin_shadow_dep::include_source!("includes/optional.inc")?;
    //~[values_external]^ builtin_type_shadow

    include!("includes/future.inc").await;
    builtin_shadow_dep::include_source!("includes/future.inc").await;
    //~[values_external]^ builtin_type_shadow

    let _ = format!("{}", include!("includes/binding.inc"));
    let _ = format!("{}", builtin_shadow_dep::include_source!("includes/binding.inc"));
    //~[values_external]^ builtin_type_shadow

    let _ = include!("includes/format_source.inc");
    let _ = builtin_shadow_dep::include_source!("includes/format_source.inc");
    //~[values_external]^ builtin_type_shadow

    let _ = 0..include!("includes/binding.inc");
    let _ = 0..builtin_shadow_dep::include_source!("includes/binding.inc");
    //~[values_external]^ builtin_type_shadow
    Some(())
}

fn desugared_pattern() {
    // The let statements keep the two invocation spans distinct after lowering.
    let _ = include!("includes/for_pattern.inc");
    let _ = builtin_shadow_dep::include_source!("includes/for_pattern.inc");
    //~[values_external]^ builtin_type_shadow
}

fn argument_and_sibling() -> usize {
    macro_rules! add {
        ($value:expr) => {
            $value + builtin_shadow_dep::include_source!("includes/number.inc")
        };
    }
    add!({
        let u8 = 7;
        //~[values,values_external]^ builtin_type_shadow
        u8
    })
}

fn forwarded_array_element() -> [usize; 2] {
    macro_rules! pass {
        ($value:expr) => {
            $value
        };
    }
    [
        include!("includes/array_argument.inc"),
        include!("includes/binding.inc"),
    ]
}

macro_rules! include_alias {
    () => {
        include!("includes/helper_alias.inc");
    };
}
use include_alias as renamed_helper;

#[proc_macro_attr::generate_shadowing_includes]
//~[types_external,values_external]^ builtin_type_shadow
//~[types_external,values_external]| builtin_type_shadow
//~[types_external,values_external]| builtin_type_shadow
//~[values_external]| builtin_type_shadow
//~[values_external]| builtin_type_shadow
struct Marker;

fn call_generated_definition() -> Option<usize> {
    generated_optional!()
}

#[proc_macro_attr::dummy]
mod authored_includes {
    use core::include as read;
    read!("includes/helper_alias.inc");
    include!("includes/alias_and_generic.inc");

    pub fn read() -> usize {
        include!("includes/binding.inc")
    }
}

#[proc_macro_attr::dummy]
macro_rules! authored_items {
    () => {
        include!("includes/alias_and_generic.inc");
    };
}

mod call_authored_definition {
    authored_items!();
}

mod forwarded_macro_name {
    macro_rules! pass {
        ($name:ident) => {
            #[proc_macro_attr::dummy]
            $name!("includes/helper_alias.inc");
        };
    }
    pass!(include);
}

#[allow(incomplete_include)]
fn root_invocations() {
    let _ = include!("includes/attributed_call.inc");
    let _ = include!("includes/trailing_comma.inc");
    let _ = include!("includes/trailing_semicolon.inc");
}

// Both functions include the same source range but resolve read! in different scopes.
fn local_reader() -> usize {
    macro_rules! read {
        () => {
            include!("binding.inc")
        };
    }
    include!("includes/reader.inc")
}

fn external_reader() -> usize {
    macro_rules! read {
        () => {
            builtin_shadow_dep::include_source!("binding.inc")
        };
    }
    include!("includes/reader.inc")
}

//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
fn forwarded_operand_and_array_sibling() -> [usize; 2] {
    macro_rules! pass {
        ($value:expr) => {
            $value + builtin_shadow_dep::include_source!("number.inc")
        };
    }
    [
        include!("includes/array_argument.inc"),
        include!("includes/binding.inc"),
    ]
}

fn main() {}

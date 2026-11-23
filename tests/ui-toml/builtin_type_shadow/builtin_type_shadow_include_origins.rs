//@aux-build:../../ui/auxiliary/builtin_shadow_dep.rs
//@aux-build:../../ui/auxiliary/proc_macro_attr.rs
//@revisions: types values types_external values_external
//@[types] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types
//@[values] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
//@[types_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types_external
//@[values_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values_external
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this binding shadows the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
#![feature(proc_macro_hygiene, stmt_expr_attributes)]
#![allow(nonstandard_style, unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]

fn authored_arguments() {
    let _ = vec![{
        type u32 = i32;
        //~^ builtin_type_shadow
        fn generic<u8>() {}
        //~^ builtin_type_shadow
        7
    }];
    let _ = vec![include!("includes/alias_expr.inc")];
    let _ = builtin_shadow_dep::forward_expr!(bare, include!("includes/alias_expr.inc"));
    let _ = builtin_shadow_dep::forward_expr!(format, include!("includes/alias_expr.inc"));
    assert!(include!("includes/binding.inc") > 0);

    let _ = proc_macro_attr::format_forged_include!(Marker);
    //~[types_external,values_external]^ builtin_type_shadow
}

macro_rules! plus_one {
    ($value:expr) => {
        $value + 1
    };
}

fn generated_arguments() {
    let _ = plus_one!(builtin_shadow_dep::include_source!("includes/binding.inc"));
    //~[values_external]^ builtin_type_shadow
    let _ = vec![builtin_shadow_dep::include_source!("includes/binding.inc")];
    //~[values_external]^ builtin_type_shadow
    let _ = dbg!(builtin_shadow_dep::include_source!("includes/binding.inc"));
    //~[values_external]^ builtin_type_shadow
    let _ = matches!(builtin_shadow_dep::include_source!("includes/binding.inc"), 17);
    //~[values_external]^ builtin_type_shadow
    assert_eq!(builtin_shadow_dep::include_source!("includes/binding.inc"), 17);
    //~[values_external]^ builtin_type_shadow
    let _ = include!("includes/wrapped_argument.inc");
}

fn repeated_reader() {
    let pair = include!("includes/shared_pair.inc");
    let first = include!("includes/shared_reader.inc");
    let second = builtin_shadow_dep::include_source!("includes/shared_reader.inc");
    //~[values_external]^ builtin_type_shadow
}

macro_rules! choose {
    () => {
        include!("binding.inc")
    };
}
#[path = "includes/shared_module.inc"]
mod direct;
// Both modules inherit the same definition before the name is shadowed below.
#[path = "includes/shared_module.inc"]
mod also_direct;

macro_rules! choose {
    () => {
        builtin_shadow_dep::include_source!("binding.inc")
    };
}
#[path = "includes/shared_module.inc"]
mod indirect;

fn replaced_expressions() {
    let direct = #[proc_macro_attr::replace_with_include("includes/binding.inc")]
    //~[values_external]^ builtin_type_shadow
    0;
    let included = include!("includes/replaced_expr.inc");
}

fn main() {}

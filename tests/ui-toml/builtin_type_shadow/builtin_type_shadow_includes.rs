//@aux-build:../../ui/auxiliary/builtin_shadow_dep.rs
//@aux-build:../../ui/auxiliary/proc_macro_attr.rs
//@revisions: types values types_external values_external
//@[types] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types
//@[values] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
//@[types_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/types_external
//@[values_external] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values_external
//@error-in-other-file: this binding shadows the built-in type `u8`
//@error-in-other-file: this generic shadows the built-in type `i8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `u32`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u16`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `u8`
//@[values,values_external] error-in-other-file: this value has the same name as the built-in type `i64`
//@[values_external] error-in-other-file: this value has the same name as the built-in type `i64`
#![feature(stmt_expr_attributes)]
#![allow(nonstandard_style, unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]

mod direct_attributes {
    include!("includes/attributes.inc");
}

mod external_attributes {
    builtin_shadow_dep::include_source!("includes/attributes.inc");
    //~[types_external,values_external]^ builtin_type_shadow
    //~[types_external,values_external]| builtin_type_shadow
}

fn nested_expressions() {
    // The extra block in the second file must not change who introduced the binding.
    let _ = include!("includes/external.inc");
    let _ = include!("includes/external_braced.inc");
    let _ = include!("includes/external_parenthesized.inc");
    macro_rules! read_external {
        ($path:literal) => {
            builtin_shadow_dep::include_source!($path)
        };
    }
    let _ = include!("includes/external_wrapped.inc");
    let _ = include!("includes/sibling.inc");
}

macro_rules! make_fn {
    ($body:block) => {
        fn authored() -> usize {
            let _ = builtin_shadow_dep::include_source!("number.inc");
            $body
        }
    };
}
// The external include above is in a sibling statement of the caller's block.
include!("includes/authored.inc");

macro_rules! render {
    ($item:item, $body:expr) => {
        $item
    };
}
// The file is included first as an item and then in the body of that very item.
include!("includes/reused.inc");

fn repeated_includes() {
    let _ = [include!("includes/binding.inc"), include!("includes/binding.inc")];
    let _ = include!("includes/binding.inc");
    let _ = include!("includes/binding.inc");
}

fn neighboring_array_elements() {
    let _: [usize; 2] = [include!("includes/sibling.inc"), include!("includes/binding.inc")];
}

fn forwarded_block() {
    macro_rules! pass {
        ($body:block) => {
            $body
        };
    }
    let _ = include!("includes/pass.inc");
    let _ = builtin_shadow_dep::include_source!("includes/pass.inc");
    //~[values_external]^ builtin_type_shadow
}

fn main() {}

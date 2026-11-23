//@rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/builtin_type_shadow/values
#![allow(unused, clippy::all)]
#![warn(clippy::builtin_type_shadow)]
#![deny(unfulfilled_lint_expectations)]

async fn parameters(
    u8: usize,
    //~^ builtin_type_shadow
    mut i8: usize,
    //~^ builtin_type_shadow
    ref u16: usize,
    //~^ builtin_type_shadow
    (i16, _): (usize, usize),
    //~^ builtin_type_shadow
    #[allow(clippy::builtin_type_shadow)] ref i32: usize,
    #[expect(clippy::builtin_type_shadow)] u32: usize,
) {
}

fn main() {
    let _closure = async |bool: usize| bool;
    //~^ builtin_type_shadow
}

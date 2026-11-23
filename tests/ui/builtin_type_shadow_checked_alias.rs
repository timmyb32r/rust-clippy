//@revisions: unchecked checked
//@aux-build:builtin_shadow_checked_dep.rs
#![cfg_attr(checked, feature(checked_type_aliases))]
#![allow(incomplete_features, non_camel_case_types)]
#![warn(clippy::builtin_type_shadow)]

mod local {
    type u8<const N: usize = 0> = core::primitive::u8;
    //~[checked]^ builtin_type_shadow

    #[cfg(unchecked)]
    const _: u8 = u8::MAX;
    #[cfg(checked)]
    const _: u8 = u8::<0>::MAX;
}

mod imported {
    use builtin_shadow_checked_dep::Byte as u8;
    //~^ builtin_type_shadow
    const _: u8 = u8::<0>::MAX;
}

mod fixed {
    type u8 = builtin_shadow_checked_dep::Byte<0>;
    const _: u8 = u8::MAX;
}

fn main() {}

#![allow(non_camel_case_types, clippy::builtin_type_shadow)]

pub mod types {
    pub use core::primitive::{u8 as Byte, u16 as u8};
}

pub mod values {
    pub fn u8() {}
}

#[macro_export]
macro_rules! declare_type {
    () => {
        pub type i64 = ::core::primitive::u64;
    };
}

#[macro_export]
macro_rules! declare_generic {
    () => {
        pub fn read<i8>(_: i8) {}
    };
}

#[macro_export]
macro_rules! declare_value {
    () => {
        pub fn u32() {}
    };
}

#[macro_export]
macro_rules! include_source {
    ($path:literal) => {
        include! { $path }
    };
}

#[macro_export]
macro_rules! forward_expr {
    (bare, $value:expr) => {
        $value
    };
    (format, $value:expr) => {
        format!("{}", $value)
    };
}

#[allow(ambiguous_glob_reexports, unused_imports)]
pub mod ambiguous {
    mod a {
        pub use core::primitive::u16 as u8;
    }
    mod b {
        pub use core::primitive::u32 as u8;
    }
    pub use a::*;
    pub use b::*;
}

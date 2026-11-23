#[proc_macro_attr::dummy]
pub type u8 = u16;

#[proc_macro_attr::dummy]
pub fn widen<i8>() {}

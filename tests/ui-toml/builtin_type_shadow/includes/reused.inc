render! {
    fn reused() -> usize {
        macro_rules! render {
            ($item:item, $body:expr) => { $body };
        }
        builtin_shadow_dep::include_source!("reused.inc")
    },
    {
        let u32 = 10;
        u32
    }
}

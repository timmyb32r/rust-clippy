{
    let first = include!("shared_reader.inc");
    let second = builtin_shadow_dep::include_source!("shared_reader.inc");
    (first, second)
}

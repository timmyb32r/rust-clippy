//@check-pass

fn main() {
    // Load the argument before the format string so its byte positions are smaller.
    let _ = include!("auxiliary/format_args_value.inc");
    let _ = include!("auxiliary/format_args_expression.inc");
}

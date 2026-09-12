macro_rules! arithmetic_virtual {
    ($in_name:ident $op:tt $second:expr => $out_name:ident) => {
        ArithmeticConditions {
            first_signal: Signal::new_virtual($in_name),
            second_signal: None,
            second_constant: Some($second as i32),
            operation: $crate::macros::arithmetic_combinator_op!($op),
            output_signal: Signal::new_virtual($out_name),
        }
    };
}

#[allow(unused_macros)]
macro_rules! log {
    ($($arg:tt)*) => {
        web_sys::console::log_1(&format!($($arg)*).into());
    };
}
macro_rules! arithmetic_combinator_op {
    (*) => {
        OP_MUL
    };
    (/) => {
        OP_DIV
    };
    (+) => {
        OP_ADD
    };
    (-) => {
        OP_SUB
    };
    (%) => {
        OP_MOD
    };
    (^) => {
        OP_POW
    };
    (<<) => {
        OP_LSHIFT
    };
    (>>) => {
        OP_RSHIFT
    };
    (AND) => {
        OP_AND
    };
    (OR) => {
        OP_OR
    };
    (XOR) => {
        OP_XOR
    };
}

pub(crate) use arithmetic_combinator_op;
pub(crate) use arithmetic_virtual;

#[allow(unused_imports)]
pub(crate) use log;

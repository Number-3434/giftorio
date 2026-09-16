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
/// Logs a message to the web console.
///
/// Alos returns back the original objects passed in, allowing this to be used inline.
macro_rules! log {
    ($($arg:tt)*) => {{
        let value = format!($($arg)*);
        web_sys::console::log_1(&value.clone().into());
        value
    }};
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

/// High-level utility macro for making wire connections.
macro_rules! get_wires {
    ($wire_type:ident $ents:ident; $src_type:ident $src_name:expr => $dst_type:ident $dst_name:expr) => {
        [
            entity_idx_by_tag!($ents, $src_name).expect("No entity found with tag"),
            get_wires!(@wire_type $wire_type, $src_type),
            entity_idx_by_tag!($ents, $dst_name).expect("No entity found with tag"),
            get_wires!(@wire_type $wire_type, $dst_type),
        ]
    };

    (@wire_type R, IN) => { 1 };
    (@wire_type G, IN) => { 2 };
    (@wire_type R, OUT) => { 3 };
    (@wire_type G, OUT) => { 4 };

    (@wire_type C, IN) => { 5 };
    (@wire_type C, OUT) => { 6 };
}
macro_rules! entity_idx_by_tag {
    ($ents:ident, $tag:expr) => {
        $ents
            .iter()
            .find(|e| e.has_tag($tag))
            .map(|e| e.entity_number)
    };
}

pub(crate) use arithmetic_combinator_op;
pub(crate) use arithmetic_virtual;
pub(crate) use entity_idx_by_tag;
pub(crate) use get_wires;

#[allow(unused_imports)]
pub(crate) use log;

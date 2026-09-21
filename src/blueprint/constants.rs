use std::sync::{Arc, LazyLock};

use crate::blueprint::models::Quality;

/// Timer entity positions.
pub const TIMER1_POS: (f64, f64) = (-2.5, -3.0);
pub const TIMER2_POS: (f64, f64) = (-1.5, -3.0);
pub const TIMER3_POS: (f64, f64) = (-1.5, -4.0);
pub const TIMER4_POS: (f64, f64) = (-1.5, -5.0);
pub const TIMER5_POS: (f64, f64) = (-2.5, -5.5);
pub const TIMER6_POS: (f64, f64) = (-1.5, -6.0);

/// Direction constants.
pub const DIR_U: u32 = 0;
pub const DIR_R: u32 = 4;
pub const DIR_D: u32 = 8;
pub const DIR_L: u32 = 12;

/// Blueprint version constant.
pub const BLUEPRINT_VERSION: u64 = 562949955518464;

/// Threshold used for binary grayscale conversion. (out of 256)
pub const GRAYSCALE_THRESH: u8 = 128;

macro_rules! lazy_const {
    ($name:ident: $type:ident = $value:expr) => {
        pub const $name: LazyLock<Arc<$type>> = LazyLock::new(|| Arc::from($value));
    };
}

lazy_const!(QUAL_NORMAL: Quality = Quality::Normal);
lazy_const!(QUAL_UNCOMMON: Quality = Quality::Uncommon);
lazy_const!(QUAL_RARE: Quality = Quality::Rare);
lazy_const!(QUAL_EPIC: Quality = Quality::Epic);
lazy_const!(QUAL_LEGENDARY: Quality = Quality::Legendary);
lazy_const!(QUAL_UNKNOWN: Quality = Quality::Unknown);

lazy_const!(DEC_CB: str = "decider-combinator");
lazy_const!(ARI_CB: str = "arithmetic-combinator");
lazy_const!(CONSTANT_COMB: str = "constant-combinator");
lazy_const!(SUBSTATION: str = "substation");
lazy_const!(LAMP: str = "small-lamp");
lazy_const!(BLUEPRINT: str = "blueprint");

lazy_const!(SIG_TYPE_VIRTUAL: str = "virtual");

lazy_const!(SIG_F: str = "signal-F");
lazy_const!(SIG_S: str = "signal-S");
lazy_const!(SIG_T: str = "signal-T");
lazy_const!(SIG_EACH: str = "signal-each");

/// Comparators
pub const COMP_GT: &'static str = ">";
pub const COMP_LT: &'static str = "<";
pub const COMP_EQ: &'static str = "=";
pub const COMP_GE: &'static str = ">=";
pub const COMP_LE: &'static str = "<=";
pub const COMP_NE: &'static str = "!=";

/// Operations
pub const OP_MUL: &'static str = "*";
pub const OP_DIV: &'static str = "/";
pub const OP_ADD: &'static str = "+";
pub const OP_SUB: &'static str = "-";
pub const OP_MOD: &'static str = "%";
pub const OP_POW: &'static str = "^";
pub const OP_LSHIFT: &'static str = "<<";
pub const OP_RSHIFT: &'static str = ">>";
pub const OP_AND: &'static str = "AND";
pub const OP_OR: &'static str = "OR";
pub const OP_XOR: &'static str = "XOR";

/// Compare types
pub const COMP_AND: &'static str = "and";

pub const WIRE_R: u32 = 1;
pub const WIRE_G: u32 = 2;
pub const WIRE_OUT_R: u32 = 3;
pub const WIRE_OUT_G: u32 = 4;
pub const WIRE_C: u32 = 5;

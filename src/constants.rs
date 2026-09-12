/// Default delay in milliseconds when a frame’s delay is zero.
pub const DEFAULT_FRAME_DELAY_MS: u32 = 100;

/// Number of milliseconds per second.
pub const MS_PER_S: f64 = 1000.0;

/// Timer entity positions.
pub const TIMER1_POS: (f64, f64) = (-2.5, -3.0);
pub const TIMER2_POS: (f64, f64) = (-1.5, -3.0);
pub const TIMER3_POS: (f64, f64) = (-1.5, -4.0);
pub const TIMER4_POS: (f64, f64) = (-1.5, -5.0);
pub const TIMER5_POS: (f64, f64) = (-2.5, -5.5);
pub const TIMER6_POS: (f64, f64) = (-1.5, -6.0);

/// Direction constants.
pub const DIR_R: u32 = 4;
pub const DIR_L: u32 = 12;

/// Blueprint version constant.
pub const BLUEPRINT_VERSION: u64 = 562949955518464;

/// Threshold used for binary grayscale conversion. (out of 256)
pub const GRAYSCALE_THRESH: u8 = 128;

/// Quality constants.
pub const QUAL_NORMAL: &str = "normal";
pub const QUAL_UNCOMMON: &'static str = "uncommon";
pub const QUAL_RARE: &'static str = "rare";
pub const QUAL_EPIC: &'static str = "epic";
pub const QUAL_LEGENDARY: &'static str = "legendary";
pub const QUAL_UNKNOWN: &'static str = "quality-unknown";
pub const QUAL_NONE: &'static str = "none";

/// Entity types
pub const DECIDER_COMB: &'static str = "decider-combinator";
pub const ARITHMETIC_COMB: &'static str = "arithmetic-combinator";
pub const CONSTANT_COMB: &'static str = "constant-combinator";
pub const SUBSTATION: &'static str = "substation";
pub const LAMP: &'static str = "small-lamp";
pub const BLUEPRINT: &'static str = "blueprint";

/// Signal types
pub const SIG_TYPE_VIRTUAL: &'static str = "virtual";

/// Signals
pub const SIG_F: &'static str = "signal-F";
pub const SIG_S: &'static str = "signal-S";
pub const SIG_T: &'static str = "signal-T";
pub const SIG_EACH: &'static str = "signal-each";

/// Comparators
pub const COMP_EQ: &'static str = "=";
pub const COMP_GE: &'static str = ">=";
pub const COMP_LT: &'static str = "<";

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

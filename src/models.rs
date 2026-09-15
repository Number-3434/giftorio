use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};

#[derive(serde::Deserialize, Clone)]
pub struct BlueprintArgs {
    pub name: String,

    #[serde(rename = "flippedAxes")]
    pub flipped_axes: FlippedAxes,
    #[serde(rename = "grayscaleBits")]
    pub grayscale_bits: u32,
    #[serde(rename = "imageRotation")]
    pub image_rotation: ImageRotation,
    #[serde(rename = "imageType")]
    pub image_type: String,
    #[serde(rename = "includeLastFrame")]
    pub last_frame: bool,
    #[serde(rename = "maxSize")]
    pub max_size: u32,
    #[serde(rename = "outputFormat")]
    pub output_format: OutputFormat,
    #[serde(rename = "useGreenLampWires")]
    pub prefer_green_wires: bool,
    #[serde(rename = "useHorizontalLampWires")]
    pub prefer_horizontal_wires: bool,
    #[serde(rename = "resamplingFilter")]
    pub sampling_filter: ResamplingFilter,
    #[serde(rename = "signalCompression")]
    pub signal_compression: Option<SignalCompression>,
    #[serde(rename = "sortSignals")]
    pub sort_signals: bool,
    #[serde(rename = "substationQuality")]
    pub substation_quality: SubstationQuality,
    #[serde(rename = "targetFps")]
    pub target_fps: u32,
    #[serde(rename = "useDLC")]
    pub use_dlc: bool,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum SubstationQuality {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "uncommon")]
    Uncommon,
    #[serde(rename = "rare")]
    Rare,
    #[serde(rename = "epic")]
    Epic,
    #[serde(rename = "legendary")]
    Legendary,
}
impl Display for SubstationQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Normal => write!(f, "normal"),
            Self::Uncommon => write!(f, "uncommon"),
            Self::Rare => write!(f, "rare"),
            Self::Epic => write!(f, "epic"),
            Self::Legendary => write!(f, "legendary"),
        }
    }
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum ImageRotation {
    #[serde(rename = "none")]
    None = 0,
    #[serde(rename = "deg90")]
    Deg90 = 90,
    #[serde(rename = "deg180")]
    Deg180 = 180,
    #[serde(rename = "deg270")]
    Deg270 = 270,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum FlippedAxes {
    #[serde(rename = "none")]
    None = 0,
    #[serde(rename = "x")]
    X = 1,
    #[serde(rename = "y")]
    Y = 2,
    #[serde(rename = "both")]
    Both = 3,
}

#[derive(serde::Deserialize, Clone, PartialEq, Eq)]
pub enum ResamplingFilter {
    #[serde(rename = "catrom")]
    Catrom,
    #[serde(rename = "gaussian")]
    Gaussian,
    #[serde(rename = "lanczos3")]
    Lanczos3,
    #[serde(rename = "nearest")]
    Nearest,
    #[serde(rename = "triangle")]
    Triangle,
}

#[derive(serde::Deserialize, Clone, PartialEq)]
pub enum SignalCompression {
    #[serde(rename = "temporal")]
    Temporal { window: u32 },
    #[serde(rename = "delta")]
    Delta,
}

#[derive(serde::Deserialize, Clone, PartialEq)]
pub enum OutputFormat {
    #[serde(rename = "blueprint")]
    Blueprint,
    #[serde(rename = "json")]
    Json,
}

#[derive(Serialize)]
pub struct Blueprint {
    pub blueprint: BlueprintInner,
}

#[derive(Serialize)]
pub struct BlueprintInner {
    pub icons: Vec<Icon>,
    pub entities: Vec<Entity>,
    pub wires: Vec<Wire>,
    pub item: &'static str,
    pub label: String,
    pub version: u64,
}

#[derive(Serialize)]
pub struct Icon {
    pub signal: Signal,
    pub index: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Signal {
    #[serde(rename = "type")]
    pub type_: Arc<String>,
    pub name: Arc<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<&'static str>,
}
impl Signal {
    pub fn new_virtual(name: &str) -> Self {
        Self {
            type_: Arc::new(SIG_TYPE_VIRTUAL.to_string()),
            name: Arc::new(name.to_string()),
            quality: None,
        }
    }
}

fn is_none_or_normal(value: &Option<String>) -> bool {
    match value {
        None => true,
        Some(s) => s == "normal",
    }
}

pub type Wire = [u32; 4];

#[derive(Clone, Serialize)]
pub struct Entity {
    pub entity_number: u32,
    pub name: &'static str,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_behavior: Option<ControlBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_description: Option<&'static str>,
    #[serde(skip_serializing_if = "is_none_or_normal")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on: Option<bool>,
    #[serde(skip)]
    pub custom_tag: Option<&'static str>,
}

impl Entity {
    /// Create a new entity with default None values for optional fields
    pub fn new(entity_number: u32, name: &'static str, position: impl Into<Position>) -> Self {
        Entity {
            entity_number,
            name,
            position: position.into(),
            direction: None,
            control_behavior: None,
            player_description: None,
            quality: None,
            always_on: None,
            custom_tag: None,
        }
    }

    pub fn has_tag(&self, tag: &'static str) -> bool {
        self.custom_tag == Some(tag)
    }

    pub fn with_always_on(mut self, always_on: bool) -> Self {
        self.always_on = Some(always_on);
        self
    }
    pub fn with_control_behavior(mut self, behavior: ControlBehavior) -> Self {
        self.control_behavior = Some(behavior);
        self
    }
    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.player_description = Some(desc);
        self
    }
    pub fn with_direction(mut self, direction: u32) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Custom tag for the entity. This does not get serialized.
    ///
    /// Utility to help identify entities for wire connections.
    pub fn with_tag(mut self, tag: &'static str) -> Self {
        self.custom_tag = Some(tag);
        self
    }
}

#[derive(Clone, Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}
impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
impl From<(f64, f64)> for Position {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum ControlBehavior {
    Constant {
        sections: Sections,
    },
    Decider {
        decider_conditions: DeciderConditions,
    },
    Arithmetic {
        arithmetic_conditions: ArithmeticConditions,
    },
    ColorLamp {
        use_colors: bool,
        color_mode: i8,
        rgb_signal: Arc<Signal>,
    },
    GrayLamp {
        use_colors: bool,
        color_mode: i8,
        red_signal: Arc<Signal>,
        green_signal: Arc<Signal>,
        blue_signal: Arc<Signal>,
    },
}
impl ControlBehavior {
    pub fn from_arithmetic_conditions(arithmetic: ArithmeticConditions) -> Self {
        Self::Arithmetic {
            arithmetic_conditions: arithmetic,
        }
    }
    pub fn from_decider_conditions(decider: DeciderConditions) -> Self {
        Self::Decider {
            decider_conditions: decider,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Sections {
    pub sections: Vec<Section>,
}

#[derive(Clone, Serialize)]
pub struct Section {
    pub index: u32,
    pub filters: Vec<Filter>,
}

#[derive(Clone, Serialize)]
pub struct Filter {
    pub index: u32,
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}
pub trait NetworkSwapper {
    fn swap_networks(&mut self);
}

impl NetworkSwapper for DeciderConditions {
    fn swap_networks(&mut self) {
        for c in self.conditions.iter_mut() {
            if let Some(n) = &mut c.first_signal_networks {
                n.red ^= true;
                n.green ^= true;
            }
        }
        self.outputs.iter_mut().for_each(|o| o.swap_networks());
    }
}
impl NetworkSwapper for CombinatorOutput {
    fn swap_networks(&mut self) {
        if let Some(n) = &mut self.networks {
            n.red ^= true;
            n.green ^= true;
        }
    }
}
#[derive(Clone, Serialize)]
pub struct DeciderConditions {
    pub conditions: Vec<Condition>,
    pub outputs: Vec<CombinatorOutput>,
}

#[derive(Clone, Serialize)]
pub struct Condition {
    pub first_signal: Signal,
    pub constant: i32,
    pub comparator: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_type: Option<&'static str>,
    pub first_signal_networks: Option<NetworkFilters>,
}

#[derive(Clone, Serialize)]
pub struct CombinatorOutput {
    pub copy_count_from_input: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<i32>,
    pub signal: Arc<Signal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<NetworkFilters>,
}
impl CombinatorOutput {
    pub fn new(signal: Arc<Signal>, constant: Option<i32>) -> Self {
        Self {
            copy_count_from_input: constant.is_none(),
            constant,
            signal,
            networks: None,
        }
    }

    pub fn with_networks(mut self, networks: NetworkFilters) -> Self {
        self.networks = Some(networks);
        self
    }
}

#[derive(Clone, Serialize)]
pub struct NetworkFilters {
    pub red: bool,
    pub green: bool,
}

#[allow(dead_code)]
impl NetworkFilters {
    pub fn all() -> Self {
        Self {
            red: true,
            green: true,
        }
    }
    pub fn green() -> Self {
        Self {
            red: false,
            green: true,
        }
    }
    pub fn none() -> Self {
        Self {
            red: false,
            green: false,
        }
    }
    pub fn red() -> Self {
        Self {
            red: true,
            green: false,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ArithmeticConditions {
    pub first_signal: Signal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_signal: Option<Signal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_constant: Option<i32>,
    pub operation: &'static str,
    pub output_signal: Signal,
}

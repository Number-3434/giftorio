use std::fmt::Display;

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
    #[serde(rename = "mode")]
    pub mode: Mode,
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
    Deg0 = 0,
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
pub enum Mode {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "lamps")]
    Lamps,
    #[serde(rename = "lampGrid")]
    LampGrid,
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

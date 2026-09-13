use crate::constants::*;
use crate::models::Signal;
use serde_json::Value;
use std::sync::Arc;

/// Enhances the provided signals by associating quality levels based on DLC usage.
///
/// # Arguments
///
/// * `use_dlc` - Whether to include additional quality levels.
/// * `signals` - A vector of signal JSON objects.
///
/// # Returns
///
/// A new vector of signal JSON objects with added quality attributes.
pub fn get_signals_with_quality(use_dlc: bool, sort: bool) -> Vec<Arc<Signal>> {
    let mut all_signals = get_signal_list(use_dlc);

    if sort {
        // Sort by total length of the type + name (so use the smallest ids first)
        all_signals
            .sort_by_key(|s| s["type"].as_str().unwrap().len() + s["name"].as_str().unwrap().len());
    }
    all_signals
        .into_iter()
        .flat_map(|signal| {
            let mut signals_vec = Vec::new();
            let qualities = if use_dlc {
                vec![
                    QUAL_NORMAL,
                    QUAL_UNCOMMON,
                    QUAL_RARE,
                    QUAL_EPIC,
                    QUAL_LEGENDARY,
                    QUAL_UNKNOWN,
                ]
            } else {
                vec![QUAL_NORMAL, QUAL_UNKNOWN]
            };
            for quality in qualities.iter() {
                let signal_type = signal["type"].as_str().unwrap();
                let signal_name = signal["name"].as_str().unwrap();

                // Skip the common signal of F, S, T as they're used internally
                if signal_type == "virtual" && *quality == QUAL_NORMAL {
                    if signal_name == SIG_F || signal_name == SIG_S || signal_name == SIG_T {
                        continue;
                    }
                }

                let signal = Arc::from(Signal {
                    type_: Arc::new(signal_type.to_string()),
                    name: Arc::new(signal_name.to_string()),
                    quality: Some(quality),
                });
                signals_vec.push(signal);
            }
            signals_vec
        })
        .collect()
}

/// Retrieves the list of signals from the embedded JSON file.
///
/// # Arguments
///
/// * `use_dlc` - Whether to include additional signals from the Space Age DLC.
///
/// # Returns
///
/// A vector of signal JSON objects.
fn get_signal_list(use_dlc: bool) -> Vec<Value> {
    let signals_json = if use_dlc {
        include_str!("data/signals-dlc.json")
    } else {
        include_str!("data/signals.json")
    };
    serde_json::from_str(signals_json).unwrap()
}

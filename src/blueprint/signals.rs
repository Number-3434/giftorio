use crate::blueprint::models::Signal;
use crate::constants::*;
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
    let mut signals: Vec<Arc<Signal>> = get_signal_list(use_dlc)
        .into_iter()
        .flat_map(|signal| {
            let mut sigs = Vec::new();
            let quals = if use_dlc {
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
            for q in quals.iter() {
                let n = signal["name"].as_str().unwrap();
                let t = signal["type"].as_str().unwrap();

                // Skip the common signal of F, S, T as they're used internally
                if t == "virtual" && *q == QUAL_NORMAL && matches!(n, SIG_F | SIG_S | SIG_T) {
                    continue;
                }
                sigs.push(Arc::from(Signal {
                    type_: Arc::new(t.to_string()),
                    name: Arc::new(n.to_string()),
                    quality: if *q == QUAL_NORMAL { None } else { Some(q) },
                }));
            }
            sigs
        })
        .collect();

    if sort {
        // Sort by total length of the type + name + quality (so use the smallest ids first)
        signals.sort_by_key(|s| {
            s.type_.len() + s.name.len() + s.quality.as_ref().unwrap_or(&"").len()
        });
    }
    signals
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
        include_str!("../data/signals-dlc.json")
    } else {
        include_str!("../data/signals.json")
    };
    serde_json::from_str(signals_json).unwrap()
}

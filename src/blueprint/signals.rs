use crate::blueprint::{constants::*, models::*};
use crate::models::SignalSorting;
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
pub fn get_signals_with_quality<'a>(args: &crate::models::BlueprintArgs) -> Vec<Arc<Signal>> {
    let quals: Vec<Arc<Quality>> = if args.use_dlc {
        vec![
            Arc::clone(&QUAL_NORMAL),
            Arc::clone(&QUAL_UNCOMMON),
            Arc::clone(&QUAL_RARE),
            Arc::clone(&QUAL_EPIC),
            Arc::clone(&QUAL_LEGENDARY),
            Arc::clone(&QUAL_UNKNOWN),
        ] as Vec<_>
    } else {
        vec![Arc::clone(&QUAL_NORMAL), Arc::clone(&QUAL_UNKNOWN)] as Vec<_>
    };
    let mut sig_types: Vec<Arc<str>> = Vec::new(); // Allow "type" field to reference the same strings
    let mut signals: Vec<_> = get_signal_list(args.use_dlc)
        .into_iter()
        .flat_map(|signal| {
            let mut sigs = Vec::new();

            let n = signal["name"].as_str().unwrap();
            let t = signal["type"].as_str().unwrap();
            let name: Arc<str> = Arc::from(n);

            // Use a singular cached String for each type instead of one per signal
            // Search from right as we added the new type at the end
            let type_ = if let Some(i) = sig_types.iter().rposition(|x| x.to_string() == t) {
                Arc::clone(&sig_types[i])
            } else {
                let type_: Arc<str> = Arc::from(t);
                sig_types.push(Arc::clone(&type_));
                type_
            };

            for q in quals.iter() {
                // Skip the common signal of F, S, T as they're used internally
                if t == "virtual"
                    && *q == *QUAL_NORMAL
                    && (n == &**SIG_F || n == &**SIG_S || n == &**SIG_T)
                {
                    continue;
                }
                sigs.push(Arc::from(Signal {
                    type_: Arc::clone(&type_),
                    name: Arc::clone(&name),
                    quality: Some(Arc::clone(&q)),
                }));
            }
            sigs
        })
        .collect();

    match args.signal_sorting {
        // Only sort by name (compressor really likes this)
        SignalSorting::Compression => signals.sort_by_key(|s| s.name.len()),
        SignalSorting::Json => signals.sort_by_key(|s| {
            // Sort by total length of the type + name + quality (actual smallest JSON)
            s.quality.as_ref().map_or(0, |q| match q.as_ref() {
                Quality::Normal => -1 * ",'quality':''".len() as i64,
                _ => q.to_string().len() as i64,
            }) + s.name.len() as i64
                + s.type_.len() as i64
                + if s.type_.as_ref() == "item" {
                    // type defaults to "item" if not specified
                    -1 * ",'type':''".len() as i64
                } else {
                    s.type_.len() as i64
                }
        }),
        _ => {}
    }

    return signals;
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

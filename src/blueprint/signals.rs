use crate::blueprint::models::*;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

thread_local! {
    pub static QUALITIES: RefCell<Vec<Arc<str>>> = const {
        RefCell::new(Vec::new())
    };
    pub static SIGNALS: RefCell<Vec<Arc<crate::blueprint::models::Signal>>> = const {
        RefCell::new(Vec::new())
    };
}

/// Enhances the provided signals by associating quality levels based on DLC usage.
///
/// # Arguments
///
/// * `signals_csv` - The raw CSV string of signals, as a Uint8Array.
/// * `qualities_csv` - Raw CSV string of qualities, as a Uint8Array.
///
/// # Returns
///
/// A new vector of signal JSON objects with added quality attributes.
pub fn load_signal_data(signals_csv: &[u8], qualities_csv: &[u8]) {
    use std::str::from_utf8;

    let mut sig_types: HashMap<Box<[u8]>, Arc<str>> = HashMap::new(); // Allow "type" field to reference the same strings
    let mut signals: Vec<Arc<Signal>> = Vec::new();
    let qualities = qualities_csv
        .split(|&b| b == b',')
        .map(|q| Arc::<str>::from(from_utf8(q).unwrap()))
        .collect::<Vec<_>>();

    for line in signals_csv.split(|&b| b == b'\n') {
        let mut fields = line.split(|&b| b == b',');
        let type_bytes = fields.next().unwrap();
        let name_bytes = fields.next().unwrap();
        let type_: Arc<str>;

        if let Some(value) = sig_types.get(type_bytes) {
            type_ = Arc::clone(value);
        } else {
            type_ = Arc::from(from_utf8(type_bytes).unwrap());
            sig_types.insert(type_bytes.into(), Arc::clone(&type_));
        }

        for q in qualities.iter() {
            signals.push(Arc::from(Signal {
                type_: Arc::clone(&type_),
                name: Arc::from(from_utf8(name_bytes).unwrap()),
                quality: Some(Arc::clone(q)),
            }));
        }
    }

    QUALITIES.with(|q| *q.borrow_mut() = qualities);
    SIGNALS.with(|s| *s.borrow_mut() = signals);
}

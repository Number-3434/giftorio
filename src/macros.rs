#[allow(unused_macros)]
/// Logs a message to the web console.
macro_rules! log {
    ($($arg:tt)*) => {{
        let value = format!($($arg)*);
        web_sys::console::log_1(&value.clone().into());
        value
    }};
}

#[allow(unused_imports)]
pub(crate) use log;

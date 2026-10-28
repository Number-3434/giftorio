use wasm_bindgen::prelude::*;

pub fn js_num(num: impl Into<f64>) -> js_sys::Number {
    js_sys::Number::from(num.into())
}

pub fn format_duration<T: Into<u128>>(ms: T) -> String {
    let ms = ms.into();

    let total_seconds = ms / 1000;

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    match hours {
        0 => format!("{minutes:02}:{seconds:02}"),
        _ => format!("{hours:02}:{minutes:02}:{seconds:02}"),
    }
}

/// Checks if the value is the default value for the type.
pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

/// Checks if an Option is None or the default value for the type.
pub fn is_none_or_default<T: Default + PartialEq>(value: &Option<T>) -> bool {
    value.as_ref().is_none_or(|v| v == &T::default())
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn wasm_shared_memory(data: &[u8]) -> Result<js_sys::Object, JsValue> {
    use crate::util::JsReflectGetSet;

    let obj = js_sys::Object::new();

    obj.reflect_set("memory", wasm_bindgen::memory())?;
    obj.reflect_set("ptr", js_num(data.as_ptr() as u32))?;
    obj.reflect_set("len", js_num(data.len() as u32))?;

    Ok(obj)
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}
impl<T: Clone> OneOrMany<T> {
    /// Returns a new clone of the contained value.
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            Self::One(v) => vec![v.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

pub trait JsReflectGetSet {
    fn reflect_get<R: JsCast>(&self, k: impl Into<JsValue>) -> Result<R, JsValue>;
    fn reflect_set(&self, k: impl Into<JsValue>, v: impl Into<JsValue>) -> Result<bool, JsValue>;
}
impl<TSelf: AsRef<JsValue>> JsReflectGetSet for TSelf {
    fn reflect_get<R: JsCast>(&self, k: impl Into<JsValue>) -> Result<R, JsValue> {
        js_sys::Reflect::get(self.as_ref(), &k.into())?.dyn_into()
    }
    fn reflect_set(&self, k: impl Into<JsValue>, v: impl Into<JsValue>) -> Result<bool, JsValue> {
        js_sys::Reflect::set(self.as_ref(), &k.into(), &v.into())
    }
}

pub trait ResultJsExt<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue>;
}
impl<T, E: std::fmt::Display> ResultJsExt<T, E> for Result<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue> {
        self.map_err(|e| JsValue::from_str(&format!("{prefix}: {e}")))
    }
}

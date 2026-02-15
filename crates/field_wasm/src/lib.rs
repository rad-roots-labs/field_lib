#![forbid(unsafe_code)]

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn app_wasm_build_info_json() -> String {
    let runtime = match radroots_field_core::RadrootsRuntime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            return format!(r#"{{\"error\":\"runtime init failed: {}\"}}"#, err);
        }
    };

    runtime.info_json()
}

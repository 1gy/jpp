use jpp_core::JsonPath;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn query(jsonpath: &str, json_str: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    let path = JsonPath::parse(jsonpath).map_err(|e| e.to_string())?;

    let results = path.query(&json);
    let output: Vec<_> = results.into_iter().cloned().collect();

    serde_json::to_string_pretty(&output).map_err(|e| format!("Serialization error: {}", e))
}

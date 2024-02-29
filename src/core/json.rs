use serde_json::Value;

pub fn extract_json_data(data: &str) -> Option<Value> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    // Parse the string of data into serde_json::Value.

    match serde_json::from_str(data) {
        Ok(n) => Some(n),
        Err(_) => None,
    }
}

pub trait JsonInterface {
    fn get_exchange_info(&self) -> Option<String>;
}

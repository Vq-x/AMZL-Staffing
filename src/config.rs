use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub target_pph: i32,
    pub total_hours: f32,
    // Add other configuration fields as needed
}

impl Default for Config {
    fn default() -> Self {
        Config {
            target_pph: 250,
            total_hours: 6.5,
        }
    }
}

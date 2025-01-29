use crate::models::BagRecord;
use csv::Reader;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub target_pph: i32,
    pub total_hours: i32,
    // Add other configuration fields as needed
}

impl Default for Config {
    fn default() -> Self {
        Config {
            target_pph: 250,
            total_hours: 6,
        }
    }
}

pub fn read_csv(file_path: &str) -> Result<Vec<BagRecord>, Box<dyn Error>> {
    let mut records: Vec<BagRecord> = Vec::new();
    let file = File::open(file_path)?;
    let mut rdr = Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: BagRecord = result?;
        records.push(record);
    }
    Ok(records)
}

pub fn read_config(file_path: &str) -> Result<Config, Box<dyn Error>> {
    if !Path::new(file_path).exists() {
        // Create the config file with default values
        let default_config = Config::default();
        let toml_string = toml::to_string(&default_config)?;
        let mut file = File::create(file_path)?;
        file.write_all(toml_string.as_bytes())?;
    }

    let config_content = fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

pub fn get_config_file_path() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("config.toml"))
}

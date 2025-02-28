use crate::models::{Algorithm, BagRecord};
use csv::Reader;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub target_pph: i32,
    pub total_hours: f32,
    pub target_hc: i32,
    pub algorithm: Algorithm,
    // Add other configuration fields as needed
}

impl Config {
    const DEFAULT_PATH: &'static str = "config.toml";

    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = Self::get_config_path()?;
        Self::read_or_create(&path)
    }

    fn read_or_create(path: &Path) -> Result<Self, Box<dyn Error>> {
        if !path.exists() {
            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            Self::create_default(path)?;
        }
        Self::from_file(path)
    }

    fn create_default(path: &Path) -> Result<(), Box<dyn Error>> {
        let default = Self::default();
        let toml = toml::to_string(&default)?;
        // Ensure directory exists before writing
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        println!("{}", path.to_str().unwrap());
        fs::write(path, toml)?;
        Ok(())
    }

    fn from_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
        Ok(dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("AMZL-Staffing")
            .join(Self::DEFAULT_PATH))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            target_pph: 250,
            total_hours: 6.5,
            target_hc: 30,
            algorithm: Algorithm::TargetPPH,
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

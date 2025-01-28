use crate::config::Config;
use crate::models::BagRecord;
use csv::Reader;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
};
use toml;

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
    if !std::path::Path::new(file_path).exists() {
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

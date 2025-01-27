use crate::models::BagRecord;
use csv::Reader;
use std::{error::Error, fs::File};

pub fn read_csv() -> Result<Vec<BagRecord>, Box<dyn Error>> {
    let mut records: Vec<BagRecord> = Vec::new();
    let file = File::open("test.csv")?;
    let mut rdr = Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: BagRecord = result?;
        records.push(record);
    }
    Ok(records)
}

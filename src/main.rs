use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::{error::Error, fs::File, process};

#[derive(Debug)]
struct SortZone {
    cluster: char,
    aisle: u32,
    level: u32,
    column: char,
}
impl SortZone {
    fn to_string(&self) -> String {
        format!(
            "{}-{}.{}{}",
            self.cluster, self.aisle, self.level, self.column
        )
    }
    fn adjacent_aisle(&self) -> u32 {
        if self.aisle % 2 == 0 {
            self.aisle - 1
        } else {
            self.aisle + 1
        }
    }
}

impl<'de> Deserialize<'de> for SortZone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SortZoneVisitor;

        impl<'de> Visitor<'de> for SortZoneVisitor {
            type Value = SortZone;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in the format 'A-1.1A'")
            }

            fn visit_str<E>(self, value: &str) -> Result<SortZone, E>
            where
                E: de::Error,
            {
                let parts: Vec<&str> = value.split('-').collect();
                if parts.len() != 2 {
                    return Err(de::Error::custom("expected format 'A-1.1A'"));
                }

                let cluster = parts[0]
                    .chars()
                    .next()
                    .ok_or_else(|| de::Error::custom("missing cluster"))?;
                let aisle_level_column: Vec<&str> = parts[1].split('.').collect();
                if aisle_level_column.len() != 2 {
                    return Err(de::Error::custom("expected format '1.1A'"));
                }

                let aisle = aisle_level_column[0]
                    .parse::<u32>()
                    .map_err(de::Error::custom)?;
                let level_column: Vec<char> = aisle_level_column[1].chars().collect();
                if level_column.len() != 2 {
                    return Err(de::Error::custom("expected format '1A'"));
                }

                let level = level_column[0]
                    .to_digit(10)
                    .ok_or_else(|| de::Error::custom("invalid level"))?
                    as u32;
                let column = level_column[1];

                Ok(SortZone {
                    cluster,
                    aisle,
                    level,
                    column,
                })
            }
        }

        deserializer.deserialize_str(SortZoneVisitor)
    }
}

#[derive(Debug, Deserialize)]
struct BagRecord {
    #[serde(rename = "Sort Zone")]
    sort_zone: SortZone,
    #[serde(rename = "Planned Bag Count")]
    planned_bag_count: i32,
    #[serde(rename = "Planned Package Count")]
    planned_package_count: i32,
}

#[derive(Debug)]
struct Aisle {
    aisle: u32,
    bag_records: Vec<BagRecord>,
}

#[derive(Debug)]
struct Cluster {
    cluster: char,
    aisles: Vec<Aisle>,
}

#[derive(Debug)]
struct Floor {
    clusters: Vec<Cluster>,
}

impl Floor {
    fn new(bags: Vec<BagRecord>) -> Self {
        let mut clusters: Vec<Cluster> = Vec::new();
        for bag in bags {
            let cluster_char = bag.sort_zone.cluster;
            let aisle_number = bag.sort_zone.aisle;

            // Find or create the cluster
            let cluster = clusters.iter_mut().find(|c| c.cluster == cluster_char);
            if let Some(cluster) = cluster {
                // Find or create the aisle within the cluster
                let aisle = cluster.aisles.iter_mut().find(|a| a.aisle == aisle_number);
                if let Some(aisle) = aisle {
                    aisle.bag_records.push(bag);
                } else {
                    cluster.aisles.push(Aisle {
                        aisle: aisle_number,
                        bag_records: vec![bag],
                    });
                }
            } else {
                // Create a new cluster and add the aisle
                clusters.push(Cluster {
                    cluster: cluster_char,
                    aisles: vec![Aisle {
                        aisle: aisle_number,
                        bag_records: vec![bag],
                    }],
                });
            }
        }

        Self { clusters }
    }
}

fn read_csv() -> Result<Vec<BagRecord>, Box<dyn Error>> {
    let mut records: Vec<BagRecord> = Vec::new();
    let file = File::open("test.csv")?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: BagRecord = result?;
        records.push(record);
    }
    Ok(records)
}

fn main() {
    if let Ok(records) = read_csv() {
        println!("{:?}", records);
        for record in &records {
            println!("{}", record.sort_zone.adjacent_aisle());
        }
        let floor = Floor::new(records);
        println!("{:?}", floor);
        println!("{:?} clusters", floor.clusters.len());
        let aisle_count = floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>();
        println!("{:?} aisles", aisle_count);
    } else {
        println!("error reading csv");
        process::exit(1);
    }
}

use crate::utils::read_config;
use once_cell::sync::Lazy;
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::fmt;

pub static TOTAL_HOURS: Lazy<i32> = Lazy::new(|| {
    read_config("config.toml")
        .expect("could not read config")
        .total_hours
});

#[derive(Debug, PartialEq, Clone)]
pub struct SortZone {
    pub cluster: char,
    pub aisle: u32,
    pub level: u32,
    pub column: char,
}

impl SortZone {
    pub fn display(&self) -> String {
        format!(
            "{}-{}.{}{}",
            self.cluster, self.aisle, self.level, self.column
        )
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
                    .ok_or_else(|| de::Error::custom("invalid level"))?;
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

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct BagRecord {
    #[serde(rename = "Sort Zone")]
    pub sort_zone: SortZone,
    #[serde(rename = "Planned Bag Count")]
    pub planned_bag_count: i32,
    #[serde(rename = "Planned Package Count")]
    pub planned_package_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Aisle {
    pub cluster: char,
    pub aisle_num: u32,
    pub bag_records: Vec<BagRecord>,
}

impl Aisle {
    pub fn total_packages(&self) -> i32 {
        self.bag_records
            .iter()
            .map(|b| b.planned_package_count)
            .sum()
    }

    pub fn get_aisle_pph(&self) -> i32 {
        self.total_packages() / *TOTAL_HOURS
    }

    pub fn display_aisle(&self) -> String {
        format!("{}-{}", self.cluster, self.aisle_num)
    }
}

#[derive(Debug)]
pub struct Cluster {
    pub cluster: char,
    pub aisles: Vec<Aisle>,
}

impl Cluster {
    pub fn get_aisle(&self, aisle: u32) -> Option<&Aisle> {
        self.aisles.iter().find(|a| a.aisle_num == aisle)
    }

    pub fn get_first_aisle(&self) -> Option<&Aisle> {
        self.aisles.iter().min_by_key(|a| a.aisle_num)
    }

    pub fn get_last_aisle(&self) -> Option<&Aisle> {
        self.aisles.iter().max_by_key(|a| a.aisle_num)
    }

    pub fn get_total_packages(&self) -> i32 {
        self.aisles.iter().map(|a| a.total_packages()).sum::<i32>()
    }

    pub fn get_next_aisle(&self, aisle: u32) -> Option<&Aisle> {
        self.aisles.iter().find(|a| a.aisle_num == aisle + 1)
    }

    pub fn get_previous_aisle(&self, aisle: u32) -> Option<&Aisle> {
        self.aisles.iter().find(|a| a.aisle_num == aisle - 1)
    }
}

#[derive(Debug)]
pub struct Floor {
    pub clusters: Vec<Cluster>,
}

impl Floor {
    pub fn new(bags: Vec<BagRecord>) -> Self {
        let mut clusters: Vec<Cluster> = Vec::new();
        for bag in bags {
            let cluster_char = bag.sort_zone.cluster;
            let aisle_number = bag.sort_zone.aisle;

            let cluster = clusters.iter_mut().find(|c| c.cluster == cluster_char);
            if let Some(cluster) = cluster {
                let aisle = cluster
                    .aisles
                    .iter_mut()
                    .find(|a| a.aisle_num == aisle_number);
                if let Some(aisle) = aisle {
                    aisle.bag_records.push(bag);
                } else {
                    cluster.aisles.push(Aisle {
                        cluster: cluster_char,
                        aisle_num: aisle_number,
                        bag_records: vec![bag],
                    });
                }
            } else {
                clusters.push(Cluster {
                    cluster: cluster_char,
                    aisles: vec![Aisle {
                        cluster: cluster_char,
                        aisle_num: aisle_number,
                        bag_records: vec![bag],
                    }],
                });
            }
        }
        clusters.iter_mut().for_each(|c| {
            c.aisles.sort_by_key(|a| a.aisle_num);
        });
        Self { clusters }
    }

    pub fn packages_per_hour(&self) -> i32 {
        self.clusters
            .iter()
            .map(|c| c.aisles.iter().map(|a| a.total_packages()).sum::<i32>())
            .sum::<i32>()
            / *TOTAL_HOURS
    }

    pub fn get_aisle_in_cluster(&self, cluster: char, aisle: u32) -> Option<&Aisle> {
        self.clusters
            .iter()
            .find(|c| c.cluster == cluster)
            .and_then(|c| c.aisles.iter().find(|a| a.aisle_num == aisle))
    }

    pub fn get_cluster(&self, cluster: char) -> Option<&Cluster> {
        self.clusters.iter().find(|c| c.cluster == cluster)
    }

    pub fn get_total_packages(&self) -> i32 {
        self.clusters
            .iter()
            .map(|c| c.get_total_packages())
            .sum::<i32>()
    }
}

#[derive(Debug, Clone)]
pub struct StowSlot<'a> {
    pub cluster: char,
    pub aisles: Vec<&'a Aisle>,
    pub is_floater: bool,
    pub pph: i32,
    pub floor: &'a Floor,
}

impl<'a> StowSlot<'a> {
    pub fn new(cluster: char, aisles: Vec<&'a Aisle>, floor: &'a Floor) -> Self {
        let mut obj = Self {
            cluster,
            aisles,
            is_floater: false,
            pph: 0,
            floor,
        };
        obj.update_pph();
        obj
    }

    pub fn add_aisle(&mut self, aisle: &'a Aisle) {
        self.aisles.push(aisle);
        self.update_pph();
    }

    pub fn add_aisle_range(&mut self, start: u32, end: u32, cluster: char) {
        for aisle in start..end {
            if let Some(aisle_ref) = self.floor.get_aisle_in_cluster(cluster, aisle) {
                self.add_aisle(aisle_ref);
            }
        }
    }

    fn update_pph(&mut self) {
        self.pph = self.aisles.iter().map(|a| a.total_packages()).sum::<i32>() / *TOTAL_HOURS;
        self.is_floater = self.pph <= 150;
    }

    pub fn display_aisles(&self) {
        for aisle in &self.aisles {
            println!("{}", aisle.display_aisle());
        }
    }

    pub fn display_aisle_range(&self) {
        println!(
            "{} - {}: {} PPH, is consecutive: {}, is floater: {}",
            self.aisles.first().unwrap().display_aisle(),
            self.aisles.last().unwrap().display_aisle(),
            self.pph,
            self.is_consecutive(),
            self.is_floater
        );
    }

    pub fn is_consecutive(&self) -> bool {
        self.aisles
            .iter()
            .zip(self.aisles.iter().skip(1))
            .all(|(a, b)| a.aisle_num + 1 == b.aisle_num)
    }
}

#[derive(Debug)]
pub struct StowSlotBuilder<'a> {
    floor: &'a Floor,
    pub stow_slots: Vec<StowSlot<'a>>,
}

impl<'a> StowSlotBuilder<'a> {
    pub fn new(floor: &'a Floor) -> Self {
        Self {
            floor,
            stow_slots: Vec::new(),
        }
    }

    pub fn get_stow_slot_from_aisle(&mut self, aisle: &Aisle) -> Option<&mut StowSlot<'a>> {
        self.stow_slots
            .iter_mut()
            .find(|s| s.aisles.iter().any(|a| *a == aisle))
    }

    pub fn display_stow_slots(&self) {
        for slot in &self.stow_slots {
            slot.display_aisle_range();
        }
    }

    pub fn total_stow_slots(&self) -> i32 {
        self.stow_slots.len() as i32
    }

    pub fn stow_slots_per_cluster(&self) {
        for cluster in &self.floor.clusters {
            println!(
                "stow slots in cluster {}: {}",
                cluster.cluster,
                self.stow_slots
                    .iter()
                    .filter(|s| s.cluster == cluster.cluster)
                    .count()
            );
        }
    }

    pub fn start_algorithm(&mut self, target_pph: i32) {
        for cluster in &self.floor.clusters {
            for aisle in &cluster.aisles {
                match cluster.get_previous_aisle(aisle.aisle_num) {
                    Some(previous_aisle) => {
                        if let Some(existing_slot) = self.get_stow_slot_from_aisle(previous_aisle) {
                            if existing_slot.pph <= target_pph {
                                existing_slot.add_aisle(aisle);
                                continue;
                            }
                        }
                        let new_slot = StowSlot::new(cluster.cluster, vec![aisle], self.floor);
                        self.stow_slots.push(new_slot);
                    }
                    None => {
                        println!(
                            "No previous aisle found for aisle number {} in cluster {}",
                            aisle.aisle_num, cluster.cluster
                        );
                        let new_slot = StowSlot::new(cluster.cluster, vec![aisle], self.floor);
                        self.stow_slots.push(new_slot);
                    }
                }
            }
        }
    }
}

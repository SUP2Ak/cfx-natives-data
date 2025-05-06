use crate::types::{GamesType, Metadata, OrganizedNatives, Native};
use std::path::Path;
use std::collections::HashMap;

pub async fn load_or_create_metadata() -> anyhow::Result<Metadata> {
    let path = Path::new("plugin/metadata.json");
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(Metadata::default())
    }
}

pub fn increment_version(version: &str) -> String {
    let mut parts: Vec<u32> = version.split('.').map(|s| s.parse().unwrap_or(0)).collect();

    if parts.len() != 3 {
        parts = vec![0, 0, 0];
    } else {
        parts[2] += 1;
    }

    format!("{}.{}.{}", parts[0], parts[1], parts[2])
}

pub enum ChangeType {
    Added(String),
    Removed(String),
    Modified {
        name: String,
        field: String,
        old_value: String,
        new_value: String,
    },
    NoChange,
}

pub fn check_and_update_natives(
    old_content: &str,
    new_content: &str,
    game_type: GamesType,
    metadata: &mut Metadata,
) -> anyhow::Result<Vec<ChangeType>> {
    println!("Checking changes for {:?}", game_type);
    println!("Old content length: {}", old_content.len());
    println!("New content length: {}", new_content.len());

    let old_natives: OrganizedNatives = serde_json::from_str(old_content).unwrap_or_default();
    let new_natives: OrganizedNatives = serde_json::from_str(new_content)?;
    let mut changes = Vec::new();

    fn compare_natives(old_vec: &[Native], new_vec: &[Native], category: &str) -> Vec<ChangeType> {
        let mut changes = Vec::new();
        let old_map: HashMap<_, _> = old_vec.iter().map(|n| (&n.name, n)).collect();
        let new_map: HashMap<_, _> = new_vec.iter().map(|n| (&n.name, n)).collect();

        for (name, new_native) in new_map.iter() {
            if let Some(old_native) = old_map.get(name) {
                let old_value = serde_json::to_value(old_native).unwrap();
                let new_value = serde_json::to_value(new_native).unwrap();

                if old_value != new_value {
                    let old_obj = old_value.as_object().unwrap();
                    let new_obj = new_value.as_object().unwrap();

                    for (field, new_field_value) in new_obj {
                        if let Some(old_field_value) = old_obj.get(field) {
                            if old_field_value != new_field_value {
                                println!("Field '{}' changed for native '{}':", field, name);
                                println!("Old: {}", old_field_value);
                                println!("New: {}", new_field_value);
                                
                                changes.push(ChangeType::Modified {
                                    name: name.to_string(),
                                    field: format!("{}.{}", category, field),
                                    old_value: old_field_value.to_string(),
                                    new_value: new_field_value.to_string(),
                                });
                            }
                        }
                    }
                }
            } else {
                changes.push(ChangeType::Added(format!("{}.{}", category, name)));
            }
        }

        for name in old_map.keys() {
            if !new_map.contains_key(name) {
                changes.push(ChangeType::Removed(format!("{}.{}", category, name)));
            }
        }

        changes
    }

    changes.extend(compare_natives(&old_natives.client, &new_natives.client, "client"));
    changes.extend(compare_natives(&old_natives.server, &new_natives.server, "server"));
    changes.extend(compare_natives(&old_natives.shared, &new_natives.shared, "shared"));

    if !changes.is_empty() {
        match game_type {
            GamesType::GTA5 => metadata.natives_gta5 = increment_version(&metadata.natives_gta5),
            GamesType::RDR3 => metadata.natives_rdr3 = increment_version(&metadata.natives_rdr3),
            _ => (),
        }
    }

    Ok(if changes.is_empty() {
        vec![ChangeType::NoChange]
    } else {
        changes
    })
}
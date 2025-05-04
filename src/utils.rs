use crate::types::{GamesType, Metadata, OrganizedNatives, Native};
use std::path::Path;
use std::collections::HashMap;

pub async fn load_or_create_metadata() -> anyhow::Result<Metadata> {
    let path = Path::new("assets/metadata.json");
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
    if old_content == new_content {
        return Ok(vec![ChangeType::NoChange]);
    }

    let old_natives: OrganizedNatives = serde_json::from_str(old_content).unwrap_or_default();
    let new_natives: OrganizedNatives = serde_json::from_str(new_content)?;
    let mut changes = Vec::new();

    fn compare_natives(old_vec: &[Native], new_vec: &[Native], category: &str) -> Vec<ChangeType> {
        let mut changes = Vec::new();
        let old_map: HashMap<_, _> = old_vec.iter().map(|n| (&n.name, n)).collect();
        let new_map: HashMap<_, _> = new_vec.iter().map(|n| (&n.name, n)).collect();

        for (name, new_native) in new_map.iter() {
            match old_map.get(name) {
                Some(old_native) => {
                    if old_native.namespace != new_native.namespace {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.namespace", category),
                            old_value: old_native.namespace.clone(),
                            new_value: new_native.namespace.clone(),
                        });
                    }
                    if old_native.hash != new_native.hash {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.hash", category),
                            old_value: old_native.hash.clone(),
                            new_value: new_native.hash.clone(),
                        });
                    }
                    if old_native.description != new_native.description {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.description", category),
                            old_value: old_native.description.clone(),
                            new_value: new_native.description.clone(),
                        });
                    }
                    if old_native.apiset != new_native.apiset {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.apiset", category),
                            old_value: old_native.apiset.clone().unwrap_or_default(),
                            new_value: new_native.apiset.clone().unwrap_or_default(),
                        });
                    }
                    if old_native.game_support != new_native.game_support {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.game_support", category),
                            old_value: old_native.game_support.clone(),
                            new_value: new_native.game_support.clone(),
                        });
                    }
                    if old_native.is_rpc != new_native.is_rpc {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.is_rpc", category),
                            old_value: old_native.is_rpc.to_string(),
                            new_value: new_native.is_rpc.to_string(),
                        });
                    }
                    if old_native.cname != new_native.cname {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.cname", category),
                            old_value: old_native.cname.clone().unwrap_or_default(),
                            new_value: new_native.cname.clone().unwrap_or_default(),
                        });
                    }
                    if old_native.params.len() != new_native.params.len() {
                        changes.push(ChangeType::Modified {
                            name: name.to_string(),
                            field: format!("{}.params", category),
                            old_value: format!("{} params", old_native.params.len()),
                            new_value: format!("{} params", new_native.params.len()),
                        });
                    } else {
                        for (i, (old_param, new_param)) in old_native.params.iter().zip(new_native.params.iter()).enumerate() {
                            if old_param.type_name != new_param.type_name {
                                changes.push(ChangeType::Modified {
                                    name: name.to_string(),
                                    field: format!("{}.params[{}]", category, i),
                                    old_value: format!("{:?}", old_param),
                                    new_value: format!("{:?}", new_param),
                                });
                            }
                        }
                    }
                }
                None => {
                    changes.push(ChangeType::Added(format!("{}.{}", category, name)));
                }
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
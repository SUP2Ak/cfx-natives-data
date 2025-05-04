mod types;
mod utils;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Deserialize)]
struct TreeResponse {
    tree: Vec<TreeItem>,
}

#[derive(Deserialize)]
struct TreeItem {
    path: String,
}

pub use types::{
    Example, GamesType, Metadata, NamespacedNatives, Native, NativeVariant, OrganizedNatives,
    Parameter,
};

pub use utils::{check_and_update_natives, increment_version, load_or_create_metadata, ChangeType};

#[derive(Clone)]
pub struct NativeParser {
    client: reqwest::Client,
}

impl NativeParser {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    // First step:
    // Get natives from url, parse them to save them in json file ( GTA5, RDR3, CFX )

    fn get_native_url(game_type: GamesType) -> &'static str {
        match game_type {
            GamesType::GTA5 => "https://static.cfx.re/natives/natives.json",
            GamesType::RDR3 => {
                "https://raw.githubusercontent.com/alloc8or/rdr3-nativedb-data/master/natives.json"
            }
            GamesType::CFX => "https://static.cfx.re/natives/natives_cfx.json",
            _ => "",
        }
    }

    fn generate_docs_url(game_type: GamesType, native_hash: &str) -> String {
        match game_type {
            GamesType::RDR3 => format!("https://alloc8or.re/rdr3/nativedb/?n={}", native_hash),
            _ => format!("https://docs.fivem.net/natives/?_{}", native_hash),
        }
    }

    fn normalize_name(name: &str) -> String {
        name.replace("_", "").replace(" ", "").to_lowercase()
    }

    fn find_markdown_name<'a>(
        normalized_name: &str,
        markdown_names: &'a [String],
    ) -> Option<&'a String> {
        markdown_names
            .iter()
            .find(|&md_name| Self::normalize_name(md_name) == normalized_name)
    }

    pub async fn fetch_natives(
        &self,
        game_type: GamesType,
        markdown_names: Vec<String>,
    ) -> anyhow::Result<Vec<Native>> {
        let url = Self::get_native_url(game_type);
        let response = self.client.get(url).send().await?;

        match game_type {
            GamesType::CFX => {
                let json: serde_json::Value = response.json().await?;
                let cfx_obj = json
                    .get("CFX")
                    .ok_or_else(|| anyhow::anyhow!("Missing CFX object"))?;

                let mut natives = Vec::with_capacity(cfx_obj.as_object().map_or(0, |m| m.len()));

                if let Some(obj) = cfx_obj.as_object() {
                    for (hash, native_value) in obj {
                        if let Ok(mut native) =
                            serde_json::from_value::<Native>(native_value.clone())
                        {
                            native.hash = hash.to_string();
                            let original_name = if native.name.is_empty() {
                                format!("N_{}", hash)
                            } else {
                                native.name.clone()
                            };

                            native.cname = Some(original_name.clone());
                            let normalized_name = Self::normalize_name(&original_name);
                            if let Some(md_name) =
                                Self::find_markdown_name(&normalized_name, &markdown_names)
                            {
                                native.name = md_name.clone();
                            } else {
                                native.name = original_name;
                            }

                            native.apiset.get_or_insert_with(|| "client".to_string());
                            native.game_support = "GTA5 | RDR3".to_string();
                            native.docs_url = Self::generate_docs_url(GamesType::CFX, hash);
                            natives.push(native);
                        }
                    }
                }
                Ok(natives)
            }
            _ => {
                let json: HashMap<String, HashMap<String, Native>> =
                    serde_json::from_str(&response.text().await?)?;
                let mut natives = Vec::new();

                for (namespace, natives_map) in json {
                    for (hash, mut native) in natives_map {
                        let original_name = if native.name.is_empty() {
                            format!("N_{}", hash)
                        } else {
                            native.name.clone()
                        };
                        native.cname = Some(original_name.clone());

                        let normalized_name = Self::normalize_name(&original_name);
                        if let Some(md_name) =
                            Self::find_markdown_name(&normalized_name, &markdown_names)
                        {
                            native.name = md_name.clone();
                        } else {
                            native.name = original_name;
                        }

                        if native.apiset.is_none() {
                            native.apiset = Some("client".to_string());
                        }
                        native.hash = hash;
                        native.namespace = namespace.clone();

                        match game_type {
                            GamesType::RDR3 => {
                                native.docs_url =
                                    Self::generate_docs_url(GamesType::RDR3, &native.hash);
                                native.game_support = "RDR3".to_string();
                            }
                            _ => {
                                native.docs_url =
                                    Self::generate_docs_url(GamesType::GTA5, &native.hash);
                                native.game_support = "GTA5".to_string();
                            }
                        }
                        natives.push(native);
                    }
                }
                Ok(natives)
            }
        }
    }

    pub async fn save_natives(
        &self,
        game_type: GamesType,
        path: &str,
        markdown_names: Vec<String>,
    ) -> anyhow::Result<usize> {
        let start_time = Instant::now();
        let natives = self.fetch_natives(game_type, markdown_names).await?;
        let end_time = Instant::now();
        println!(
            "[{:?}] Time taken to fetch natives: {:?} | {:?} natives fetched",
            game_type,
            end_time.duration_since(start_time),
            natives.len()
        );
        let start_time_save = Instant::now();
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &natives)?;
        let end_time_save = Instant::now();
        println!(
            "[{:?}] Time taken to save natives: {:?} | {:?} natives saved",
            game_type,
            end_time_save.duration_since(start_time_save),
            natives.len()
        );
        Ok(natives.len())
    }

    // Second step:
    // Organize natives by apiset and get if native is rpc or not
    // Save them in different files ( GTA5, RDR3 )

    pub async fn organize_natives_by_apiset(
        &self,
        metadata: &mut Metadata,
    ) -> anyhow::Result<(usize, usize)> {
        let gta5_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.gta5.json")?;
        let rdr3_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.rdr3.json")?;
        let cfx_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.cfx.json")?;
        let old_rdr3 = std::fs::read_to_string("assets/natives.rdr3.json").unwrap_or_default();
        let mut old_gta5 = std::fs::read_to_string("assets/natives.gta5.json").unwrap_or_default();

        let simulate_change = std::env::args().any(|arg| arg == "--simulate-change");
        if simulate_change {
            println!("Simulating changes by adding a test native to GTA5...");
            if let Ok(mut content) = serde_json::from_str::<OrganizedNatives>(&old_gta5) {
                let test_native = Native {
                    name: "TEST_SIMULATED_NATIVE".to_string(),
                    hash: "0xTEST1234".to_string(),
                    namespace: "TEST".to_string(),
                    apiset: Some("client".to_string()),
                    params: vec![],
                    return_type: "void".to_string(),
                    description: "".to_string(),
                    jhash: Some("0xTEST1234".to_string()),
                    examples: None,
                    game_support: "GTA5".to_string(),
                    docs_url: "https://docs.fivem.net/natives/?_=0xTEST1234".to_string(),
                    is_rpc: false,
                    cname: None,
                };
                
                content.client.push(test_native);
                if let Some(native) = content.client.get_mut(0) {
                    native.namespace = "TEST_MODIFIED".to_string();
                }

                if let Some(_) = content.server.get_mut(50) {
                    content.server.remove(50);
                }
                
                old_gta5 = serde_json::to_string(&content)?;
            }
        }

        let mut gta5_organized = OrganizedNatives {
            client: gta5_natives,
            server: Vec::new(),
            shared: Vec::new(),
        };

        let mut rdr3_organized = OrganizedNatives {
            client: rdr3_natives,
            server: Vec::new(),
            shared: Vec::new(),
        };

        let gta5_client_names: HashSet<_> = gta5_organized
            .client
            .iter()
            .map(|n| n.name.clone())
            .collect();

        let rdr3_client_names: HashSet<_> = rdr3_organized
            .client
            .iter()
            .map(|n| n.name.clone())
            .collect();

        for native in cfx_natives {
            let mut processed_native = native.clone();
            let is_rpc = gta5_client_names.contains(&native.name)
                || rdr3_client_names.contains(&native.name);

            processed_native.is_rpc = is_rpc;

            match native.apiset.as_deref() {
                Some("server") => {
                    gta5_organized.server.push(processed_native.clone());
                    rdr3_organized.server.push(processed_native);
                }
                Some("shared") => {
                    gta5_organized.shared.push(processed_native.clone());
                    rdr3_organized.shared.push(processed_native);
                }
                _ => {
                    gta5_organized.client.push(processed_native.clone());
                    rdr3_organized.client.push(processed_native);
                }
            }
        }

        for organized in [&mut gta5_organized, &mut rdr3_organized] {
            for category in [
                &mut organized.client,
                &mut organized.server,
                &mut organized.shared,
            ] {
                category.sort_by(|a, b| a.namespace.cmp(&b.namespace).then(a.name.cmp(&b.name)));
                category.dedup_by(|a, b| a.hash == b.hash);
            }
        }

        let mut gta5_output = Vec::new();
        let mut rdr3_output = Vec::new();
        serde_json::to_writer_pretty(&mut gta5_output, &gta5_organized)?;
        serde_json::to_writer_pretty(&mut rdr3_output, &rdr3_organized)?;

        let new_gta5 = String::from_utf8(gta5_output)?;
        let new_rdr3 = String::from_utf8(rdr3_output)?;
        let gta5_changes = check_and_update_natives(&old_gta5, &new_gta5, GamesType::GTA5, metadata)?;
        let rdr3_changes = check_and_update_natives(&old_rdr3, &new_rdr3, GamesType::RDR3, metadata)?;

        for change in &gta5_changes {
            match change {
                ChangeType::Added(name) => println!("GTA5: Nouvelle native ajoutée: {}", name),
                ChangeType::Removed(name) => println!("GTA5: Native supprimée: {}", name),
                ChangeType::Modified { name, field, old_value, new_value } => {
                    println!("GTA5: Native '{}' modifiée: {} changé de '{}' à '{}'", 
                        name, field, old_value, new_value);
                }
                ChangeType::NoChange => println!("GTA5: Aucun changement"),
            }
        }

        for change in &rdr3_changes {
            match change {
                ChangeType::Added(name) => println!("RDR3: Nouvelle native ajoutée: {}", name),
                ChangeType::Removed(name) => println!("RDR3: Native supprimée: {}", name),
                ChangeType::Modified { name, field, old_value, new_value } => {
                    println!("RDR3: Native '{}' modifiée: {} changé de '{}' à '{}'", 
                        name, field, old_value, new_value);
                }
                ChangeType::NoChange => println!("RDR3: Aucun changement"),
            }
        }

        std::fs::write("assets/natives.gta5.json", &new_gta5)?;
        std::fs::write("assets/natives.rdr3.json", &new_rdr3)?;

        Ok((
            gta5_organized.client.len() + gta5_organized.server.len() + gta5_organized.shared.len(),
            rdr3_organized.client.len() + rdr3_organized.server.len() + rdr3_organized.shared.len(),
        ))
    }

    fn load_natives_from_file(&self, path: &str) -> anyhow::Result<Vec<Native>> {
        let file = std::fs::File::open(path)?;
        let natives: Vec<Native> = serde_json::from_reader(file)?;
        Ok(natives)
    }

    pub async fn fetch_markdown_filenames(
        &self,
        game_type: GamesType,
    ) -> anyhow::Result<Vec<String>> {
        let api_url = match game_type {
            GamesType::GTA5 => {
                "https://api.github.com/repos/citizenfx/natives/git/trees/master?recursive=1"
            }
            GamesType::CFX => {
                "https://api.github.com/repos/citizenfx/fivem/git/trees/master?recursive=1"
            }
            _ => return Err(anyhow::anyhow!("Unsupported game type for markdown docs")),
        };

        let response = self
            .client
            .get(api_url)
            .header("User-Agent", "request")
            .send()
            .await?
            .json::<TreeResponse>()
            .await?;

        let markdown_files: Vec<String> = response
            .tree
            .into_iter()
            .filter_map(|item| {
                if item.path.ends_with(".md") {
                    if game_type == GamesType::CFX {
                        if item.path.starts_with("ext/native-decls/") {
                            let filename = item.path.split('/').last()?;
                            Some(filename.trim_end_matches(".md").to_string())
                        } else {
                            None
                        }
                    } else {
                        let filename = item.path.split('/').last()?;
                        Some(filename.trim_end_matches(".md").to_string())
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(markdown_files)
    }
}

mod types;
use reqwest;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;
pub use types::{
    Example, GamesType, NamespacedNatives, Native, NativeVariant, OrganizedNatives, Parameter,
};

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
        }
    }

    fn generate_docs_url(game_type: GamesType, native_hash: &str) -> String {
        match game_type {
            GamesType::RDR3 => format!("https://alloc8or.re/rdr3/nativedb/?n={}", native_hash),
            _ => format!("https://docs.fivem.net/natives/?_{}", native_hash),
        }
    }

    pub async fn fetch_natives(&self, game_type: GamesType) -> anyhow::Result<Vec<Native>> {
        let url = Self::get_native_url(game_type);
        let response = self.client.get(url).send().await?;

        match game_type {
            GamesType::CFX => {
                // Parse directement en Value pour éviter une double désérialisation
                let json: serde_json::Value = response.json().await?;
                let cfx_obj = json
                    .get("CFX")
                    .ok_or_else(|| anyhow::anyhow!("Missing CFX object"))?;

                // Pré-allouer le vecteur
                let mut natives = Vec::with_capacity(cfx_obj.as_object().map_or(0, |m| m.len()));

                if let Some(obj) = cfx_obj.as_object() {
                    for (hash, native_value) in obj {
                        if let Ok(mut native) =
                            serde_json::from_value::<Native>(native_value.clone())
                        {
                            native.hash = hash.to_string();
                            if native.name.is_empty() {
                                native.name = format!("N_{}", hash);
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
                        if native.name.is_empty() {
                            native.name = format!("N_{}", hash);
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

    pub async fn save_natives(&self, game_type: GamesType, path: &str) -> anyhow::Result<usize> {
        let start_time = Instant::now();
        let natives = self.fetch_natives(game_type).await?;
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

    pub async fn organize_natives_by_apiset(&self) -> anyhow::Result<(usize, usize)> {
        let gta5_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.gta5.json")?;
        let rdr3_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.rdr3.json")?;
        let cfx_natives: Vec<Native> = self.load_natives_from_file("fetched/natives.cfx.json")?;

        std::fs::create_dir_all("organized")?;

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

        let gta5_file = std::fs::File::create("organized/natives.gta5.json")?;
        serde_json::to_writer_pretty(gta5_file, &gta5_organized)?;

        let rdr3_file = std::fs::File::create("organized/natives.rdr3.json")?;
        serde_json::to_writer_pretty(rdr3_file, &rdr3_organized)?;

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
}

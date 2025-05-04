use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamesType {
    GTA5,
    RDR3,
    CFX,
    Global,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Parameter {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub type_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Example {
    pub lang: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NativeVariant {
    // Format CFX
    Cfx {
        name: String,
        params: Vec<Parameter>,
        #[serde(rename = "results")]
        return_type: String,
        description: String,
        examples: Vec<Example>,
        hash: String,
        #[serde(rename = "ns")]
        namespace: String,
        apiset: String,
    },
    // Format GTA5
    Gta5 {
        name: String,
        params: Vec<Parameter>,
        #[serde(rename = "results")]
        return_type: String,
        description: String,
        examples: Vec<Example>,
        hash: String,
        #[serde(rename = "ns")]
        namespace: String,
        jhash: Option<String>,
    },
    // Format RDR3
    Rdr3 {
        name: String,
        comment: String,
        params: Vec<Parameter>,
        return_type: String,
        build: String,
        gta_hash: String,
        gta_jhash: Option<String>,
    },
    Global {
        name: String,
        // TODO
    }
}

// Structure unifiée pour la sortie
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Native {
    pub cname: Option<String>,
    pub name: String,
    pub params: Vec<Parameter>,
    #[serde(alias = "results", alias = "return_type")]
    pub return_type: String,
    #[serde(alias = "comment")]
    pub description: String,
    pub hash: String,
    #[serde(alias = "ns")]
    pub namespace: String,
    pub apiset: Option<String>,
    pub jhash: Option<String>,
    pub examples: Option<serde_json::Value>,
    #[serde(default = "default_docs_url")]
    pub docs_url: String,
    #[serde(default = "default_game_support")]
    pub game_support: String,
    #[serde(default = "default_is_rpc")]
    pub is_rpc: bool,
}

fn default_game_support() -> String {
    "".to_string()
}

fn default_docs_url() -> String {
    "".to_string()
}

fn default_is_rpc() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeCFX {
    name: String,
    params: Vec<Parameter>,
    results: String,
    description: String,
    examples: Vec<String>,
    ns: String,
    apiset: Option<String>,
    #[serde(default = "default_docs_url")]
    docs_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NamespacedNatives {
    #[serde(flatten)]
    pub namespaces: HashMap<String, HashMap<String, NativeVariant>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OrganizedNatives {
    pub client: Vec<Native>,
    pub server: Vec<Native>,
    pub shared: Vec<Native>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub natives_gta5: String,
    pub natives_rdr3: String,
    pub global: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            natives_gta5: "0.0.0".to_string(),
            natives_rdr3: "0.0.0".to_string(),
            global: "0.0.0".to_string(),
        }
    }
}

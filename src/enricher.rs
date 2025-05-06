use crate::types::{Native, LanguageSpecific, EnrichedParameter, ReturnInfo};
use std::collections::HashMap;

pub trait NativeEnricher {
    fn enrich(&mut self);
}

impl NativeEnricher for Native {
    fn enrich(&mut self) {
        let mut language_specifics = HashMap::new();
        language_specifics.insert("csharp".to_string(), generate_csharp_specifics(self));
        language_specifics.insert("javascript".to_string(), generate_js_specifics(self));
        language_specifics.insert("lua".to_string(), generate_lua_specifics(self));
        
        self.language_specifics = Some(language_specifics);
    }
}


fn map_type_for_csharp(type_name: &str) -> String {
    match type_name.trim_end_matches('*').to_lowercase().as_str() {
        "integer" | "int" | "long" | "vehicle" | "ped" | "entity" | "blip" => "int".to_string(),
        "float" | "double" => "float".to_string(),
        "string" | "char" => "string".to_string(),
        "boolean" | "bool" => "bool".to_string(),
        "hash" => "uint".to_string(),
        "vector3" => "Vector3".to_string(),
        _ => type_name.to_string(),
    }
}

fn map_type_for_js(type_name: &str) -> String {
    match type_name.trim_end_matches('*').to_lowercase().as_str() {
        "integer" | "int" | "long" | "hash" | "vehicle" | "ped" | "entity" | "blip" => "number".to_string(),
        "float" | "double" => "number".to_string(),
        "string" | "char" => "string".to_string(),
        "boolean" | "bool" => "boolean".to_string(),
        "vector3" => "[x: number, y: number, z: number]".to_string(),
        _ => type_name.to_string(),
    }
}

fn map_type_for_lua(type_name: &str) -> String {
    match type_name.trim_end_matches('*').to_lowercase().as_str() {
        "integer" | "int" | "long" | "hash" | "vehicle" | "ped" | "entity" | "blip" => "integer".to_string(),
        "float" | "double" => "float".to_string(),
        "string" | "char" => "string".to_string(),
        "boolean" | "bool" => "boolean".to_string(),
        "vector3" => "vector3".to_string(),
        _ => type_name.to_string(),
    }
}

fn generate_csharp_specifics(native: &Native) -> LanguageSpecific {    
    let params = native.params.as_ref().map_or(Vec::new(), |params| {
        params.iter().map(|p| {
            EnrichedParameter {
                name: p.name.clone(),
                param_type: map_type_for_csharp(&p.type_name),
                modifier: if p.type_name.ends_with('*') {
                    Some("ref".to_string())
                } else {
                    None
                },
            }
        }).collect()
    });

    LanguageSpecific {
        params,
        return_type: Some(map_type_for_csharp(&native.return_type)),
        returns: None,
    }
}

fn generate_js_specifics(native: &Native) -> LanguageSpecific {    
    let mut returns = Vec::new();
    
    if native.return_type.to_lowercase() != "void" {
        returns.push(ReturnInfo {
            name: None,
            type_name: map_type_for_js(&native.return_type),
        });
    }

    let params = native.params.as_ref().map_or(Vec::new(), |params| {
        params.iter()
            .filter(|p| !p.type_name.ends_with('*'))
            .map(|p| EnrichedParameter {
                name: p.name.clone(),
                param_type: map_type_for_js(&p.type_name),
                modifier: None,
            })
            .collect()
    });

    if let Some(params) = &native.params {
        for param in params {
            if param.type_name.ends_with('*') {
                returns.push(ReturnInfo {
                    name: Some(param.name.clone()),
                    type_name: map_type_for_js(&param.type_name),
                });
            }
        }
    }

    LanguageSpecific {
        params,
        return_type: None,
        returns: if returns.is_empty() { None } else { Some(returns) },
    }
}

fn generate_lua_specifics(native: &Native) -> LanguageSpecific {
    let mut returns = Vec::new();
    
    if native.return_type.to_lowercase() != "void" {
        returns.push(ReturnInfo {
            name: None,
            type_name: map_type_for_lua(&native.return_type),
        });
    }

    let params = native.params.as_ref().map_or(Vec::new(), |params| {
        params.iter()
            .filter(|p| !p.type_name.ends_with('*'))
            .map(|p| EnrichedParameter {
                name: p.name.clone(),
                param_type: map_type_for_lua(&p.type_name),
                modifier: None,
            })
            .collect()
    });

    if let Some(params) = &native.params {
        for param in params {
            if param.type_name.ends_with('*') {
                returns.push(ReturnInfo {
                    name: Some(param.name.clone()),
                    type_name: map_type_for_lua(&param.type_name),
                });
            }
        }
    }

    LanguageSpecific {
        params,
        return_type: None,
        returns: if returns.is_empty() { None } else { Some(returns) },
    }
}

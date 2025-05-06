use crate::types::{Native, OrganizedNatives};
use std::fs;
use std::path::Path;

pub struct LuaDocGenerator;

impl LuaDocGenerator {
    pub fn new() -> Self {
        Self
    }

    fn sanitize_lua_keyword(name: &str) -> String {
        match name.to_lowercase().as_str() {
            "end" => "_end".to_string(),
            "repeat" => "_repeat".to_string(),
            "until" => "_until".to_string(),
            "function" => "_function".to_string(),
            "local" => "_local".to_string(),
            "in" => "_in".to_string(),
            "do" => "_do".to_string(),
            "then" => "_then".to_string(),
            "else" => "_else".to_string(),
            "elseif" => "_elseif".to_string(),
            "break" => "_break".to_string(),
            "return" => "_return".to_string(),
            "while" => "_while".to_string(),
            "for" => "_for".to_string(),
            "if" => "_if".to_string(),
            _ => name.to_string(),
        }
    }

    fn is_output_parameter(type_name: &str) -> bool {
        type_name.ends_with('*')
    }

    fn format_type(type_name: &str) -> String {
        let base_type = type_name.trim_end_matches('*');
        match base_type.to_lowercase().as_str() {
            "integer" | "int" | "long" => "integer".to_string(),
            "float" | "double" => "float".to_string(),
            "string" | "char" => "string".to_string(),
            "boolean" | "bool" => "boolean".to_string(),
            "any" => "unknown".to_string(),
            "vehicle" | "ped" | "entity" | "hash" | "player" | "cam" => "integer".to_string(),
            "object" => "table".to_string(),
            "vector3" => "vector3".to_string(),
            _ => base_type.to_string(), 
        }
    }

    fn format_param_name(name: &str) -> String {
        if name.to_lowercase() == "param" || name.to_lowercase().contains("param") {
            "p".to_string()
        } else {
            Self::sanitize_lua_keyword(name)
        }
    }

    fn get_best_example(native: &Native) -> Option<String> {
        if let Some(examples) = &native.examples {
            if let Some(examples_array) = examples.as_array() {
                if let Some(lua_example) = examples_array.iter().find(|e| {
                    e.get("lang").and_then(|l| l.as_str()) == Some("lua")
                }) {
                    if let Some(code) = lua_example.get("code").and_then(|c| c.as_str()) {
                        let formatted_code = code.lines()
                            .map(|line| format!("---{}", line))
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Some(format!("---```lua\n{}\n---```", formatted_code));
                    }
                }
                else if let Some(js_example) = examples_array.iter().find(|e| {
                    e.get("lang").and_then(|l| l.as_str()) == Some("js")
                }) {
                    if let Some(code) = js_example.get("code").and_then(|c| c.as_str()) {
                        let formatted_code = code.lines()
                            .map(|line| format!("---{}", line))
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Some(format!("---```js\n{}\n---```", formatted_code));
                    }
                }
                else if let Some(cs_example) = examples_array.iter().find(|e| {
                    e.get("lang").and_then(|l| l.as_str()) == Some("cs")
                }) {
                    if let Some(code) = cs_example.get("code").and_then(|c| c.as_str()) {
                        let formatted_code = code.lines()
                            .map(|line| format!("---{}", line))
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Some(format!("---```cs\n{}\n---```", formatted_code));
                    }
                }
            }
        }
        None
    }

    fn generate_param_doc(param: &crate::types::Parameter, _: &Native) -> Option<String> {
        if Self::is_output_parameter(&param.type_name) {
            None
        } else {
            Some(format!(
                "---@param {} {}",
                Self::format_param_name(&param.name),
                Self::format_type(&param.type_name)
            ))
        }
    }

    fn generate_return_type_doc(native: &Native) -> Vec<String> {
        let mut returns = Vec::new();
        if native.return_type.to_lowercase() != "void" {
            returns.push(Self::format_type(&native.return_type));
        }
        
        if let Some(params) = &native.params {
            for param in params {
                if Self::is_output_parameter(&param.type_name) {
                    returns.push(Self::format_type(&param.type_name));
                }
            }
        }
        
        if returns.is_empty() {
            Vec::new()
        } else {
            vec![format!("---@return {}", returns.join(", "))]
        }
    }
    
    fn generate_native_doc(&self, native: &Native, api_set: &str) -> String {
        let mut doc = Vec::new();
        let mut header = format!("---**`{}`", native.namespace);
        if native.is_rpc {
            header.push_str(" `RPC`");
        }
        header.push_str(&format!(" `{}`**<br>", api_set));
        doc.push(header);
        doc.push(format!(
            "--- [Native Documentation]({})<br>",
            native.docs_url
        ));
        
        if !native.description.is_empty() {
            for line in native.description.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    doc.push(format!("--- {}", trimmed));
                }
            }
        }

        if let Some(example) = Self::get_best_example(native) {
            doc.push(example);
        }

        if let Some(params) = &native.params {
            for param in params {
                if let Some(param_doc) = Self::generate_param_doc(param, native) {
                    doc.push(param_doc);
                }
            }
        }
        doc.extend(Self::generate_return_type_doc(native));
        
        let params_str = native.params.as_ref().map_or(String::new(), |params| {
            params
                .iter()
                .filter(|p| !Self::is_output_parameter(&p.type_name))
                .map(|p| Self::format_param_name(&p.name))
                .collect::<Vec<_>>()
                .join(", ")
        });
        
        doc.push(format!("function {}({}) end", native.name, params_str));
        doc.push(String::new());
        doc.push(String::new());
        
        doc.join("\n")
    }

    pub fn generate_lua_docs(
        &self,
        natives: &OrganizedNatives,
        output_dir: &str,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(output_dir)?;
        let header = concat!("---@meta\n\n",);
        let mut client_content = header.to_string();
        for native in &natives.client {
            client_content.push_str(&self.generate_native_doc(native, "client"));
        }
        for native in &natives.shared {
            client_content.push_str(&self.generate_native_doc(native, "shared"));
        }
        fs::write(Path::new(output_dir).join("client.lua"), client_content)?;

        let mut server_content = header.to_string();
        for native in &natives.server {
            server_content.push_str(&self.generate_native_doc(native, "server"));
        }
        for native in &natives.shared {
            server_content.push_str(&self.generate_native_doc(native, "shared"));
        }
        fs::write(Path::new(output_dir).join("server.lua"), server_content)?;

        Ok(())
    }
}

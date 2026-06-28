use std::collections::HashMap;
use std::path::Path;
use crate::mra_defs::{RawMraClass, PropertyInfo, ClassInfo};

#[derive(serde::Deserialize)]
struct RawDefinitionEnum {
    edt: String,
    descriptions: Option<crate::mra_defs::RawMraLocalizedString>,
}

#[derive(serde::Deserialize)]
struct RawDefinition {
    #[serde(rename = "enum")]
    def_enum: Option<Vec<RawDefinitionEnum>>,
}

#[derive(serde::Deserialize)]
struct RawDefinitionsFile {
    definitions: HashMap<String, RawDefinition>,
}

fn resolve_candidates(
    val: &serde_json::Value,
    defs_map: &HashMap<String, Vec<(String, String, String)>>,
    out: &mut Vec<(String, String, String)>
) {
    if let Some(obj) = val.as_object() {
        // 1. Resolve $ref references from definitions.json
        if let Some(r) = obj.get("$ref").and_then(|r| r.as_str()) {
            if let Some(def_name) = r.split('/').last() {
                if let Some(candidates) = defs_map.get(def_name) {
                    out.extend(candidates.clone());
                }
            }
        }
        
        // 2. Resolve inline enum arrays
        if let Some(enum_arr) = obj.get("enum").and_then(|e| e.as_array()) {
            for item in enum_arr {
                if let Some(item_obj) = item.as_object() {
                    if let Some(edt) = item_obj.get("edt").and_then(|e| e.as_str()) {
                        let hex = edt.trim_start_matches("0x").to_uppercase();
                        let descriptions = item_obj.get("descriptions");
                        let name_ja = descriptions
                            .and_then(|d| d.get("ja"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let name_en = descriptions
                            .and_then(|d| d.get("en"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        out.push((hex, name_ja, name_en));
                    }
                }
            }
        }

        for (_, v) in obj {
            resolve_candidates(v, defs_map, out);
        }
    } else if let Some(arr) = val.as_array() {
        for v in arr {
            resolve_candidates(v, defs_map, out);
        }
    }
}

#[derive(Clone)]
pub struct MraDatabase {
    pub classes: HashMap<(u8, u8), ClassInfo>,
}

impl MraDatabase {
    pub fn load_empty() -> Self {
        Self { classes: HashMap::new() }
    }

    pub fn load() -> Self {
        let mut db = Self {
            classes: HashMap::new(),
        };
        
        let base_path = Path::new("assets/mra/MRA_v1.4.0");
        if !base_path.exists() {
            return db;
        }

        // Load definitions.json
        let mut defs_map = HashMap::new();
        let defs_path = base_path.join("definitions").join("definitions.json");
        if let Ok(content) = std::fs::read_to_string(&defs_path) {
            if let Ok(raw_file) = serde_json::from_str::<RawDefinitionsFile>(&content) {
                for (def_name, def) in raw_file.definitions {
                    if let Some(enums) = def.def_enum {
                        let mut candidates = Vec::new();
                        for e in enums {
                            let hex = e.edt.trim_start_matches("0x").to_string();
                            let name_ja = e.descriptions.as_ref().map(|d| d.ja.clone()).unwrap_or_default();
                            let name_en = e.descriptions.as_ref().map(|d| d.en.clone()).unwrap_or_default();
                            candidates.push((hex, name_ja, name_en));
                        }
                        defs_map.insert(def_name, candidates);
                    }
                }
            }
        }

        // Helper to construct PropertyInfo with resolved value candidates
        let make_property_info = |p: crate::mra_defs::RawMraProperty, defs_map: &HashMap<String, Vec<(String, String, String)>>| {
            let epc = u8::from_str_radix(p.epc.trim_start_matches("0x"), 16).unwrap_or(0);
            let mut edt_candidates = Vec::new();
            if let Some(ref data_val) = p.data {
                resolve_candidates(data_val, defs_map, &mut edt_candidates);
            }
            let info = PropertyInfo {
                epc,
                name_ja: p.property_name.ja,
                name_en: p.property_name.en,
                description_ja: p.descriptions.as_ref().map(|d| d.ja.clone()),
                description_en: p.descriptions.as_ref().map(|d| d.en.clone()),
                edt_candidates,
            };
            (epc, info)
        };

        // 1. Load super class (0x0000)
        let super_class_path = base_path.join("superClass").join("0x0000.json");
        let super_class_props = if let Ok(content) = std::fs::read_to_string(&super_class_path) {
            if let Ok(raw_class) = serde_json::from_str::<RawMraClass>(&content) {
                raw_class.el_properties.into_iter().map(|p| {
                    make_property_info(p, &defs_map)
                }).collect::<HashMap<u8, PropertyInfo>>()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        // Helper to load a class from a file path
        let load_class_file = |path: &Path| -> Option<RawMraClass> {
            let content = std::fs::read_to_string(path).ok()?;
            serde_json::from_str::<RawMraClass>(&content).ok()
        };

        // 2. Load node profile (0x0EF0)
        let node_profile_path = base_path.join("nodeProfile").join("0x0EF0.json");
        if let Some(raw_class) = load_class_file(&node_profile_path) {
            let mut props = super_class_props.clone();
            for p in raw_class.el_properties {
                let (epc, info) = make_property_info(p, &defs_map);
                props.insert(epc, info);
            }
            db.classes.insert((0x0E, 0xF0), ClassInfo {
                group_code: 0x0E,
                class_code: 0xF0,
                name_ja: raw_class.class_name.ja,
                name_en: raw_class.class_name.en,
                properties: props,
            });
        }

        // 3. Load device classes
        let devices_dir = base_path.join("devices");
        if let Ok(entries) = std::fs::read_dir(devices_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(raw_class) = load_class_file(&path) {
                        let eoj_hex = raw_class.eoj.trim_start_matches("0x");
                        if eoj_hex.len() == 4 {
                            if let (Ok(g), Ok(c)) = (
                                u8::from_str_radix(&eoj_hex[0..2], 16),
                                u8::from_str_radix(&eoj_hex[2..4], 16)
                            ) {
                                let mut props = super_class_props.clone();
                                for p in raw_class.el_properties {
                                    let (epc, info) = make_property_info(p, &defs_map);
                                    props.insert(epc, info);
                                }
                                db.classes.insert((g, c), ClassInfo {
                                    group_code: g,
                                    class_code: c,
                                    name_ja: raw_class.class_name.ja,
                                    name_en: raw_class.class_name.en,
                                    properties: props,
                                });
                            }
                        }
                    }
                }
            }
        }

        db
    }
}

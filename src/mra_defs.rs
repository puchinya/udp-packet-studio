use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RawMraLocalizedString {
    pub ja: String,
    pub en: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMraProperty {
    pub epc: String,
    #[serde(rename = "propertyName")]
    pub property_name: RawMraLocalizedString,
    #[serde(rename = "shortName")]
    pub short_name: String,
    pub descriptions: Option<RawMraLocalizedString>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMraClass {
    pub eoj: String,
    #[serde(rename = "className")]
    pub class_name: RawMraLocalizedString,
    #[serde(rename = "shortName")]
    pub short_name: String,
    #[serde(rename = "elProperties")]
    pub el_properties: Vec<RawMraProperty>,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub epc: u8,
    pub name_ja: String,
    pub name_en: String,
    pub description_ja: Option<String>,
    pub description_en: Option<String>,
    pub edt_candidates: Vec<(String, String, String)>,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub group_code: u8,
    pub class_code: u8,
    pub name_ja: String,
    pub name_en: String,
    pub properties: HashMap<u8, PropertyInfo>,
}

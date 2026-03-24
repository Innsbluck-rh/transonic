use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenSubsonicExtension {
    pub name: String,
    pub versions: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerFeatures {
    pub open_subsonic: bool,
    pub raw_extensions: Vec<OpenSubsonicExtension>,
    pub extensions: BTreeMap<String, Vec<u32>>,
}

impl ServerFeatures {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_extensions(
        open_subsonic: bool,
        raw_extensions: Vec<OpenSubsonicExtension>,
    ) -> Self {
        let extensions = raw_extensions
            .iter()
            .map(|extension| (extension.name.clone(), extension.versions.clone()))
            .collect();

        Self {
            open_subsonic,
            raw_extensions,
            extensions,
        }
    }

    pub fn has_extension(&self, name: &str) -> bool {
        self.extensions.contains_key(name)
    }
}

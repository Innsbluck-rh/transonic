use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupExportResult {
    pub path: String,
    pub profile_count: u32,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupImportResult {
    pub path: String,
    pub profile_count: u32,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupImportRequest {
    #[serde(default)]
    pub replace_existing: bool,
}

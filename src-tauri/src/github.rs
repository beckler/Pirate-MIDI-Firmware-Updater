use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{device::ConnectedDevice, validation::is_name_compatible};

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct Release {
    pub url: String,
    pub html_url: String,
    pub assets_url: String,
    pub upload_url: String,
    pub tarball_url: Option<String>,
    pub zipball_url: Option<String>,
    pub discussion_url: Option<String>,
    pub id: u64,
    pub node_id: String,
    pub tag_name: String,
    pub target_commitish: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub published_at: Option<String>,
    pub assets: Vec<Asset>,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct Asset {
    pub url: String,
    pub browser_download_url: String,
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub label: Option<String>,
    pub state: String,
    pub content_type: String,
    pub size: u64,
    pub download_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl Asset {
    pub fn is_compatible(&self, device: &ConnectedDevice) -> bool {
        is_name_compatible(device, &self.name, false)
    }
}

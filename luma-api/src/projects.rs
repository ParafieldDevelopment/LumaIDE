use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecentProjects {
    pub map: HashMap<String, Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub project_uuid: String,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<u64>,
    /// Project type id chosen in the New Workspace wizard (e.g. "luau", "roblox").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    /// Runtime id chosen for the project type (e.g. "lua-5.4").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    /// Template id the workspace was created from (e.g. "hello-world").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

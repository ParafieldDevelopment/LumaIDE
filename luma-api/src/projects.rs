use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RecentProjects {
    pub map: HashMap<String, Project>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub project_uuid: String,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
}
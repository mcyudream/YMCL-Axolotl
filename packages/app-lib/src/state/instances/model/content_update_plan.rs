use serde::{Deserialize, Serialize};

use super::ContentOwnershipKind;
use crate::state::ContentProvider;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentUpdateScope {
    All,
    UserAdded,
    Pack,
    Item,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdatePlanAction {
    pub content_id: String,
    pub relative_path: Option<String>,
    pub ownership_kind: ContentOwnershipKind,
    pub provider: ContentProvider,
    pub current_release_id: Option<String>,
    pub target_release_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdatePlan {
    pub id: String,
    pub instance_id: String,
    pub revision: u64,
    pub scope: ContentUpdateScope,
    pub actions: Vec<ContentUpdatePlanAction>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentUpdateResolutionChoice {
    KeepOverride,
    RestorePackDefault,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdateResolution {
    pub content_id: String,
    pub choice: ContentUpdateResolutionChoice,
}

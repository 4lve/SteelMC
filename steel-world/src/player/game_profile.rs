use serde::{Deserialize, Serialize};
use steel_protocol::packets::login::GameProfileProperty;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameProfileAction {
    ForcedNameChange,
    UsingBannedSkin,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GameProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
    #[serde(rename = "profileActions")]
    pub profile_actions: Option<Vec<GameProfileAction>>,
}

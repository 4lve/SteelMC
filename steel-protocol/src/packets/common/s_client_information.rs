use steel_macros::{ReadFrom, ServerPacket};

#[derive(ReadFrom, Clone, Debug)]
pub enum ChatVisiblity {
    Full = 0,
    System = 1,
    Hidden = 2,
}

#[derive(ReadFrom, Clone, Debug)]
pub enum HumanoidArm {
    Left = 0,
    Right = 1,
}

#[derive(ReadFrom, Clone, Debug)]
pub enum ParticleStatus {
    All = 0,
    Depraced = 1,
    Minimal = 2,
}

#[derive(ReadFrom, ServerPacket, Clone, Debug)]
pub struct SClientInformation {
    #[read_as(as = "string", bound = 16)]
    pub language: String,
    #[read_as(as = "var_int")]
    pub view_distance: i32,
    pub chat_visibility: ChatVisiblity,
    pub chat_colors: bool,
    #[read_as(as = "var_int")]
    pub model_customisation: i32,
    pub main_hand: HumanoidArm,
    pub text_filtering_enabled: bool,
    pub allows_listing: bool,
    pub particle_status: ParticleStatus,
}

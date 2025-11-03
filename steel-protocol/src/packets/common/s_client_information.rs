use steel_macros::PacketRead;

#[derive(PacketRead, Clone, Debug)]
pub enum ChatVisiblity {
    FULL = 0,
    SYSTEM = 1,
    HIDDEN = 2,
}

#[derive(PacketRead, Clone, Debug)]
pub enum HumanoidArm {
    LEFT = 0,
    RIGHT = 1,
}

#[derive(PacketRead, Clone, Debug)]
pub enum ParticleStatus {
    ALL = 0,
    DECREASED = 1,
    MINIMAL = 2,
}

#[derive(PacketRead, Clone, Debug)]
pub struct SClientInformationPacket {
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

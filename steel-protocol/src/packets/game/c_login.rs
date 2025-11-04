use steel_macros::{ClientPacket, WriteTo};
use steel_registry::packets::play::C_LOGIN;
use steel_utils::{BlockPos, ResourceLocation, types::GameType};

use crate::packet_traits::WriteTo;

#[derive(Clone, Debug, WriteTo)]
pub struct CommonPlayerSpawnInfo {
    #[write_as(as = "var_int")]
    pub dimension_type: i32,
    pub dimension: ResourceLocation,
    pub seed: i64,
    #[write_as(as = "byte")]
    pub game_type: GameType,
    #[write_as(as = "option_byte")]
    pub previous_game_type: Option<GameType>,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<(ResourceLocation, BlockPos)>,
    #[write_as(as = "var_int")]
    pub portal_cooldown: i32,
    #[write_as(as = "var_int")]
    pub sea_level: i32,
}

impl WriteTo for BlockPos {
    fn write(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        self.as_i64().write(writer)?;
        Ok(())
    }
}

impl WriteTo for (ResourceLocation, BlockPos) {
    fn write(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        self.0.write(writer)?;
        self.1.write(writer)?;
        Ok(())
    }
}

#[derive(ClientPacket, WriteTo, Clone, Debug)]
#[packet_id(Play = "C_LOGIN")]
pub struct CLogin {
    pub player_id: i32,
    pub hardcore: bool,
    #[write_as(as = "vec")]
    pub levels: Vec<ResourceLocation>,
    #[write_as(as = "var_int")]
    pub max_players: i32,
    #[write_as(as = "var_int")]
    pub chunk_radius: i32,
    #[write_as(as = "var_int")]
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
    pub common_player_spawn_info: CommonPlayerSpawnInfo,
    pub enforces_secure_chat: bool,
}

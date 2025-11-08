use std::sync::Arc;

use crate::player::Player;

pub trait WorldServer {
    fn add_player(&self, player: Arc<Player>);
}

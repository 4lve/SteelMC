use crate::player::Player;

pub trait WorldServer {
    fn add_player(&self, player: Player);
}

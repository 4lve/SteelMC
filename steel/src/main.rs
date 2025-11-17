use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use steel::SteelServer;
use steel_utils::{Identifier, translations};

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let mut steel = SteelServer::new().await;

    log::info!(
        "{:?}",
        steel
            .server
            .registry
            .items
            .get_tag(&Identifier::vanilla_static("swords"))
            .unwrap()
            .iter()
            .map(|b| b.key.path.to_string())
            .collect::<Vec<String>>()
    );

    log::info!(
        "{}",
        translations::DEATH_ATTACK_ANVIL_PLAYER
            .message(["4LVE", "Borrow Checker"])
            .format()
    );

    /*
    steel.server.worlds[0]
        .chunk_map
        .schedule_generation_task(ChunkStatus::Full, ChunkPos(Vector2::new(0, 0)))
        .await;

    steel.server.worlds[0]
        .chunk_map
        .run_generation_tasks()
        .await; */

    steel.start().await;
}

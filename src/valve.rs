use crate::error::Result;
use a2s::A2SClient;
use log::{debug, info};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Player {
    pub name: String,
}

#[derive(Debug)]
pub struct ValveInformation {
    pub server_name: String,
    pub version: String,
    pub player_count: u8,
    pub max_player_count: u8,
    pub players: Vec<Player>,
}

impl ValveInformation {
    pub fn new(
        server_name: String,
        version: String,
        player_count: u8,
        max_player_count: u8,
        players: Vec<Player>,
    ) -> Self {
        Self {
            server_name,
            version,
            player_count,
            max_player_count,
            players,
        }
    }
}

pub async fn retrieve_valve_info(
    a2s_client: &A2SClient,
    addr: &SocketAddr,
) -> Result<ValveInformation> {
    debug!("A2S info query to {}", addr);

    let server_info = a2s_client.info(addr).await?;
    // TODO: Valheim currently does not report any meaningful player information :(
    // let player_info = a2sClient.players(addr).await?;

    Ok(ValveInformation::new(
        server_info.name,
        server_info
            .extended_server_info
            .keywords
            .unwrap_or_else(|| "n/a".to_string()),
        server_info.players,
        server_info.max_players,
        vec![],
    ))
}

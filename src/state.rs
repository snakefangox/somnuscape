use redis::AsyncCommands;

use crate::player::Player;


#[derive(Debug, Clone)]
pub struct State {
    client: redis::Client,
}

impl State {
    pub fn new() -> anyhow::Result<Self> {
        let client = redis::Client::open(std::env::var("REDIS_URL")?)?;
        Ok(Self { client })
    }

    pub async fn con(&self) -> redis::aio::Connection {
        self.client
            .get_async_connection()
            .await
            .expect("Redis should be available")
    }

    pub async fn get_player(&self, name: &str) -> Option<Player> {
        let json: String = self.con().await.get(format!("player:{name}")).await.ok()?;
        serde_json::from_str(&json).ok()
    }

    pub async fn set_player(&self, player: Player) {
        let name = &player.name;
        self.con()
            .await
            .set::<String, String, ()>(
                format!("player:{name}"),
                serde_json::to_string(&player).unwrap(),
            )
            .await
            .unwrap();
    }
}
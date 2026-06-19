use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

#[derive(Clone)]
pub struct AppConfig {
    pub port: String,
    pub enable_bgpalerts: bool,
    pub matrix_homeserver: String,
    pub matrix_username: String,
    pub matrix_password: String,
    pub matrix_room_id: OwnedRoomId,
    pub matrix_allowed_user: OwnedUserId,
    pub asf_bot_name: String,
    pub asf_ipc_password: String,
    pub asf_base_url: String,
    pub data_dir: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            data_dir: std::env::var("DOLLY_DATA_DIR").unwrap_or_else(|_| ".".to_string()),
            port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()),
            enable_bgpalerts: std::env::var("ENABLE_BGPALERTS")
                .map(|v| v == "true")
                .unwrap_or(false),
            matrix_homeserver: std::env::var("MATRIX_HOMESERVER")?,
            matrix_username: std::env::var("MATRIX_USERNAME")?,
            matrix_password: std::env::var("MATRIX_PASSWORD")?,
            matrix_room_id: std::env::var("MATRIX_ROOM_ID")?.parse()?,
            matrix_allowed_user: std::env::var("MATRIX_ALLOWED_USER")?.parse()?,
            asf_bot_name: std::env::var("ASF_BOT_NAME").unwrap_or_else(|_| "default".to_string()),
            asf_ipc_password: std::env::var("ASF_IPC_PASSWORD").unwrap_or_default(),
            asf_base_url: std::env::var("ASF_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:1242".to_string()),
        })
    }
}

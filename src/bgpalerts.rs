use std::sync::Arc;

use axum::{extract::Json, http::StatusCode};
use matrix_sdk::{
    Client,
    ruma::{OwnedRoomId, events::room::message::RoomMessageEventContent},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize)]
pub struct BgpWebhook {
    pub message: Option<String>,
    #[serde(rename = "AlertType")]
    pub alert_type: Option<String>,
    #[serde(rename = "UnstableData")]
    pub unstable_data: Option<std::collections::HashMap<String, Value>>,
}

#[derive(Clone)]
pub struct BgpAlertsState {
    pub client: Client,
    pub room_id: OwnedRoomId,
}

pub async fn handle_webhook(
    axum::extract::State(state): axum::extract::State<Arc<BgpAlertsState>>,
    Json(payload): Json<BgpWebhook>,
) -> StatusCode {
    info!(
        alert_type = ?payload.alert_type,
        message = ?payload.message,
        unstable_data = ?payload.unstable_data,
        "Received bgp.tools webhook"
    );

    let room = state.client.get_room(&state.room_id);
    match room {
        Some(room) => {
            let asn = payload
                .unstable_data
                .as_ref()
                .and_then(|d| d.get("ASN"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let message = payload.message.as_deref().unwrap_or("");
            let content = RoomMessageEventContent::text_html(
                format!("[bgp.tools Alert] for AS{asn}\n{message}"),
                format!("<strong>[bgp.tools Alert]</strong> for AS{asn}<br>{message}"),
            );

            if let Err(e) = room.send(content).await {
                error!("Failed to send Matrix message: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        None => {
            error!("Room not found: {}", state.room_id);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    StatusCode::OK
}

use std::{collections::HashSet, sync::Arc};

use anyhow::Context;
use matrix_sdk::{
    Client, Room,
    ruma::events::room::message::{MessageType, RoomMessageEventContent, SyncRoomMessageEvent},
};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{error, warn};

use crate::config::AppConfig;

fn seen_events_path(config: &AppConfig) -> String {
    format!("{}/seen_events.json", config.data_dir)
}

struct TwoFaState {
    config: AppConfig,
    seen_events: Mutex<HashSet<String>>,
}

pub async fn register(client: &Client, config: Arc<AppConfig>) -> anyhow::Result<()> {
    let seen_events = load_seen_events(&config).await?;

    let shared = Arc::new(TwoFaState {
        config: (*config).clone(),
        seen_events: Mutex::new(seen_events),
    });

    client.add_event_handler({
        let shared = Arc::clone(&shared);
        move |event: SyncRoomMessageEvent, room: Room| {
            let shared = Arc::clone(&shared);
            async move {
                if let Err(err) = handle(event, room, shared).await {
                    error!("2fa command failed: {err:#}");
                }
            }
        }
    });

    Ok(())
}

async fn load_seen_events(config: &AppConfig) -> anyhow::Result<HashSet<String>> {
    let path = seen_events_path(config);
    if std::path::Path::new(&path).exists() {
        let data = tokio::fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&data)?)
    } else {
        Ok(HashSet::new())
    }
}

async fn handle(
    event: SyncRoomMessageEvent,
    room: Room,
    state: Arc<TwoFaState>,
) -> anyhow::Result<()> {
    let Some(event) = event.as_original() else {
        return Ok(());
    };

    let event_id = event.event_id.to_string();
    {
        let mut seen = state.seen_events.lock().await;
        if !seen.insert(event_id.clone()) {
            warn!(event_id = %event_id, "Ignoring duplicate Matrix event");
            return Ok(());
        }

         // Persist seen events after adding new one
         if let Err(e) = save_seen_events(&seen, &state.config).await {
             warn!("Failed to save seen events: {}", e);
         }
    }

    let MessageType::Text(text) = &event.content.msgtype else {
        return Ok(());
    };

    if text.body.trim() != "/2fa" {
        return Ok(());
    }

    if event.sender.as_str() != state.config.matrix_allowed_user.as_str() {
        warn!(sender = %event.sender, allowed_user = %state.config.matrix_allowed_user, "Ignoring unauthorized /2fa request");
        return Ok(());
    }

    let bot_name = state.config.asf_bot_name.trim();
    let url = format!(
        "{}/Api/Bot/{bot_name}/TwoFactorAuthentication/Token",
        state.config.asf_base_url.trim_end_matches('/')
    );

    let response = reqwest::Client::new()
        .get(url)
        .header("Authentication", &state.config.asf_ipc_password)
        .send()
        .await
        .context("requesting 2fa token")?;

    let status = response.status();
    let body = response.text().await.context("reading 2fa response body")?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("2fa request failed: {status} {body}"));
    }

    let code = extract_code(&body).context("parsing 2fa code")?;
    room.send(RoomMessageEventContent::text_plain(format!(
        "{bot_name}: {code}"
    )))
    .await?;

    Ok(())
}

fn extract_code(body: &str) -> anyhow::Result<String> {
    let parsed: Value = serde_json::from_str(body)?;
    let result = parsed
        .get("Result")
        .and_then(Value::as_object)
        .context("missing result object")?;

    let (_, token) = result.iter().next().context("missing bot entry")?;
    token
        .get("Result")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .context("missing 2fa code")
}

async fn save_seen_events(events: &HashSet<String>, config: &AppConfig) -> anyhow::Result<()> {
    let path = seen_events_path(config);
    let data = serde_json::to_string(events)?;
    tokio::fs::write(&path, data).await?;
    Ok(())
}

use axum::{routing::post, Router};
use matrix_sdk::{authentication::matrix::MatrixSession, config::RequestConfig, Client};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::info;

mod bgpalerts;
mod config;
mod matrix;

use bgpalerts::{handle_webhook, BgpAlertsState};
use config::AppConfig;
use matrix::MatrixRuntime;

/// Data needed to restore a Matrix session
#[derive(Debug, Serialize, Deserialize)]
struct PersistedSession {
    homeserver: String,
    session: MatrixSession,
}

fn session_file_path(config: &AppConfig) -> String {
    format!("{}/matrix_session.json", config.data_dir)
}

fn store_path(config: &AppConfig) -> String {
    format!("{}/matrix_store.sqlite", config.data_dir)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dolly=info".into()),
        )
        .init();

    let config = AppConfig::from_env()?;

    // Try to restore session from file
    let session_path = session_file_path(&config);
    let store_path = store_path(&config);

    // Ensure data directory exists
    std::fs::create_dir_all(&config.data_dir).ok();

    let client = if Path::new(&session_path).exists() {
        info!("Session file found, attempting to restore...");
        match restore_session(&config).await {
            Ok(client) => {
                info!("Successfully restored Matrix session");
                client
            }
            Err(err) => {
                info!("Failed to restore session: {}", err);
                // Clear both session file and store
                fs::remove_file(&session_path).await.ok();
                std::fs::remove_file(&store_path).ok();
                std::fs::remove_file(format!("{}.wal", store_path)).ok();
                std::fs::remove_file(format!("{}.shm", store_path)).ok();
                create_and_login_client(&config).await?
            }
        }
    } else {
        info!("No session file found, logging in...");
        create_and_login_client(&config).await?
    };

    let runtime = MatrixRuntime::new(client.clone(), config.clone());

    info!("Starting feature loops...");
    runtime.register().await?;

    // Do an initial sync to get the latest sync token
    info!("Performing initial sync...");
    runtime.initial_sync().await?;

    let server_task = if config.enable_bgpalerts {
        info!("BGP Alerts enabled, starting HTTP server");
        let webhook_state = std::sync::Arc::new(BgpAlertsState {
            client: runtime.client.clone(),
            room_id: config.matrix_room_id.clone(),
        });

        let app = Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(webhook_state);

        let addr = format!("0.0.0.0:{}", config.port);
        info!("Listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        Some(async { axum::serve(listener, app).await })
    } else {
        info!("BGP Alerts disabled (ENABLE_BGPALERTS not set to true)");
        None
    };

    let since = runtime.initial_sync().await?;

    if let Some(server) = server_task {
        let (_server_result, _) = tokio::join!(server, runtime.sync_forever(since));
    } else {
        runtime.sync_forever(since).await;
    }

    Ok(())
}

async fn restore_session(config: &AppConfig) -> anyhow::Result<Client> {
    let session_path = session_file_path(config);
    let session_data = fs::read_to_string(&session_path).await?;
    let persisted: PersistedSession = serde_json::from_str(&session_data)?;

    if persisted.homeserver != config.matrix_homeserver {
        return Err(anyhow::anyhow!(
            "Homeserver mismatch: stored {} != config {}",
            persisted.homeserver,
            config.matrix_homeserver
        ));
    }

    if persisted.session.meta.user_id.to_string() != config.matrix_username {
        return Err(anyhow::anyhow!(
            "User mismatch: stored {} != config {}",
            persisted.session.meta.user_id,
            config.matrix_username
        ));
    }

    let client = Client::builder()
        .homeserver_url(&config.matrix_homeserver)
        .sqlite_store(&store_path(config), None)
        .request_config(RequestConfig::new().disable_retry())
        .build()
        .await?;

    client.restore_session(persisted.session).await?;
    Ok(client)
}

async fn create_and_login_client(config: &AppConfig) -> anyhow::Result<Client> {
    let store_path = store_path(config);
    info!(
        "Initializing new Matrix client with SQLite store at {}",
        store_path
    );

    // Clean up any existing store files
    std::fs::remove_file(&store_path).ok();
    std::fs::remove_file(format!("{}.wal", store_path)).ok();
    std::fs::remove_file(format!("{}.shm", store_path)).ok();

    let client = Client::builder()
        .homeserver_url(&config.matrix_homeserver)
        .sqlite_store(&store_path, None)
        .request_config(RequestConfig::new().disable_retry())
        .build()
        .await?;

    info!("Logging in as {}", config.matrix_username);
    client
        .matrix_auth()
        .login_username(&config.matrix_username, &config.matrix_password)
        .initial_device_display_name("dolly")
        .send()
        .await?;

    info!("Logged in successfully");

    // Save the session for future restarts
    if let Some(session) = client.matrix_auth().session() {
        let persisted = PersistedSession {
            homeserver: config.matrix_homeserver.clone(),
            session: session.clone(),
        };
        let session_json = serde_json::to_string(&persisted)?;
        let session_path = session_file_path(config);
        fs::write(&session_path, session_json).await?;
        info!("Session persisted to {}", session_path);
    }

    Ok(client)
}

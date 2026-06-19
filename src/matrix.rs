pub mod commands;

use std::sync::Arc;

use matrix_sdk::{config::SyncSettings, Client};
use tracing::warn;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct MatrixRuntime {
    pub client: Client,
    pub config: AppConfig,
}

impl MatrixRuntime {
    pub fn new(client: Client, config: AppConfig) -> Self {
        Self { client, config }
    }

    pub async fn register(&self) -> anyhow::Result<()> {
        commands::two_fa::register(&self.client, Arc::new(self.config.clone())).await
    }

    pub async fn initial_sync(&self) -> anyhow::Result<String> {
        let settings = SyncSettings::default()
            .timeout(std::time::Duration::from_secs(5))
            .full_state(false);

        let response = self.client.sync_once(settings).await?;
        Ok(response.next_batch)
    }

    pub async fn sync_forever(&self, since: String) {
        let settings = SyncSettings::default()
            .token(since)
            .timeout(std::time::Duration::from_secs(30));

        if let Err(err) = self.client.sync(settings).await {
            warn!("Matrix sync error: {err:#}");
        }
    }
}

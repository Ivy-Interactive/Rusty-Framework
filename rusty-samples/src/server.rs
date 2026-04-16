use rusty::prelude::*;

use crate::app_shell::AppShell;

pub struct SamplesServer;

impl SamplesServer {
    pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Rusty Samples on port {port}");
        RustyServer::new(port, || AppShell).serve().await
    }
}

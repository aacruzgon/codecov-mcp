mod codecov_client;
mod config;
mod error;
mod heuristics;
mod models;
mod resources;
mod server;
mod tools;

use std::sync::Arc;

use config::Config;
use server::CodecovMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let config = Config::from_env()?;
    let server = CodecovMcpServer::new(Arc::new(config));
    let transport = rmcp::transport::stdio();
    rmcp::serve_server(server, transport).await?;

    Ok(())
}

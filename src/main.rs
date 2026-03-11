use std::sync::Arc;

use codecov_mcp::{
    codecov_client::CodecovClient, config::Config, server::CodecovMcpServer,
};

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
    let client = CodecovClient::new(&config)?;
    let server = CodecovMcpServer::new(Arc::new(client));
    let transport = rmcp::transport::stdio();
    let running = rmcp::serve_server(server, transport).await?;
    running.waiting().await?;

    Ok(())
}

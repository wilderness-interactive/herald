mod config;
mod google_ads;
mod google_auth;
mod server;

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("herald=debug")
        .init();

    let config = config::load_config("herald.toml")?;
    tracing::info!(
        customer_id = %config.ads.customer_id,
        "Herald config loaded"
    );

    let api = server::ApiConnection {
        http: reqwest::Client::new(),
        google_config: config.google,
        ads_config: config.ads,
    };

    let mcp_server = server::HeraldServer::new(api);
    let service = mcp_server
        .serve(rmcp::transport::io::stdio())
        .await
        .inspect_err(|e| tracing::error!("Herald MCP error: {e}"))?;

    tracing::info!("Herald running on stdio");
    service.waiting().await?;

    Ok(())
}

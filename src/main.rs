mod atrium;
mod auth_flow;
mod config;
mod google_ads;
mod google_analytics;
mod google_auth;
mod server;

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).is_some_and(|a| a == "auth") {
        return auth_flow::run("herald.toml").await;
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("herald=debug")
        .init();

    let config = config::load_config("herald.toml")?;
    let account_names: Vec<&str> = config.account.iter().map(|a| a.name.as_str()).collect();
    tracing::info!(accounts = ?account_names, "Herald config loaded");

    let api = server::ApiConnection {
        http: reqwest::Client::new(),
        google_config: config.google,
        ads_config: config.ads,
        accounts: config.account,
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

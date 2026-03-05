use std::sync::Arc;
use tokio::sync::RwLock;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};

use crate::config::{AccountConfig, AdsGlobalConfig, GoogleConfig};
use crate::google_ads;
use crate::google_auth;

// -- Shared connection data --

pub struct ApiConnection {
    pub http: reqwest::Client,
    pub google_config: GoogleConfig,
    pub ads_config: AdsGlobalConfig,
    pub accounts: Vec<AccountConfig>,
}

// -- Tool parameter structs --

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListAccountsParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChangeHistoryParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Number of days to look back for changes (7, 14, or 30)")]
    pub days_back: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PerformanceParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct KeywordParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchTermsParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GaqlParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Raw GAQL query string to execute against the Google Ads API")]
    pub query: String,
}

// -- MCP Server --

#[derive(Clone)]
pub struct HeraldServer {
    api: Arc<RwLock<ApiConnection>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HeraldServer {
    pub fn new(api: ApiConnection) -> Self {
        Self {
            api: Arc::new(RwLock::new(api)),
            tool_router: Self::tool_router(),
        }
    }

    async fn get_token(&self) -> Result<String, McpError> {
        let api = self.api.read().await;
        google_auth::fetch_access_token(&api.http, &api.google_config)
            .await
            .map_err(|e| McpError::internal_error(format!("Auth failed: {e}"), None))
    }

    async fn resolve_account(&self, name: &str) -> Result<String, McpError> {
        let api = self.api.read().await;
        api.accounts
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .map(|a| a.customer_id.clone())
            .ok_or_else(|| {
                let available: Vec<&str> = api.accounts.iter().map(|a| a.name.as_str()).collect();
                McpError::invalid_params(
                    format!("Unknown account '{name}'. Available: {available:?}"),
                    None,
                )
            })
    }

    async fn run_gaql(&self, account: &str, gaql: &str) -> Result<serde_json::Value, McpError> {
        let customer_id = self.resolve_account(account).await?;
        let token = self.get_token().await?;
        let api = self.api.read().await;
        google_ads::query(&api.http, &api.ads_config, &customer_id, &token, gaql)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))
    }

    #[tool(description = "List all configured Google Ads accounts. Use this first to see which accounts are available.")]
    async fn list_accounts(
        &self,
        Parameters(_): Parameters<ListAccountsParams>,
    ) -> Result<CallToolResult, McpError> {
        let api = self.api.read().await;
        let lines: Vec<String> = api
            .accounts
            .iter()
            .map(|a| format!("- {} ({})", a.name, a.customer_id))
            .collect();
        Ok(CallToolResult::success(vec![Content::text(
            format!("Configured accounts:\n{}", lines.join("\n")),
        )]))
    }

    #[tool(description = "List recent changes made to a Google Ads account. Shows what changed, when, by whom, and the before/after values. Use this to find when modifications were made so you can compare performance before and after.")]
    async fn list_changes(
        &self,
        Parameters(ChangeHistoryParams { account, days_back }): Parameters<ChangeHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let gaql = google_ads::change_history_query(days_back);
        let data = self.run_gaql(&account, &gaql).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get campaign-level performance metrics for a date range. Returns impressions, clicks, CTR, CPC, conversions, cost, and conversion value for each campaign. Use this to compare before/after periods around a change.")]
    async fn get_performance(
        &self,
        Parameters(PerformanceParams { account, date_from, date_to }): Parameters<PerformanceParams>,
    ) -> Result<CallToolResult, McpError> {
        let gaql = google_ads::campaign_performance_query(&date_from, &date_to);
        let data = self.run_gaql(&account, &gaql).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get keyword-level performance data for a date range. Returns keyword text, match type, quality score, impressions, clicks, CTR, CPC, conversions, and cost. Use this to analyze which keywords are performing and whether intent has improved.")]
    async fn get_keywords(
        &self,
        Parameters(KeywordParams { account, date_from, date_to }): Parameters<KeywordParams>,
    ) -> Result<CallToolResult, McpError> {
        let gaql = google_ads::keyword_performance_query(&date_from, &date_to);
        let data = self.run_gaql(&account, &gaql).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get search term report for a date range. Shows the actual search queries that triggered ads, which campaign/ad group they matched, and their performance. Use this to evaluate search intent quality and find negative keyword opportunities.")]
    async fn get_search_terms(
        &self,
        Parameters(SearchTermsParams { account, date_from, date_to }): Parameters<SearchTermsParams>,
    ) -> Result<CallToolResult, McpError> {
        let gaql = google_ads::search_terms_query(&date_from, &date_to);
        let data = self.run_gaql(&account, &gaql).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Execute a raw GAQL (Google Ads Query Language) query against an account. Use this for custom analysis when the other tools don't cover what you need.")]
    async fn run_query(
        &self,
        Parameters(GaqlParams { account, query }): Parameters<GaqlParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.run_gaql(&account, &query).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for HeraldServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Herald — sovereign ad intelligence. \
                 Pulls Google Ads data for analysis across multiple accounts: change history, \
                 campaign performance, keyword metrics, search terms, and raw GAQL queries. \
                 Use list_accounts first to see available accounts, then pass the account name to other tools."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

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
use crate::google_analytics;
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

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AnalyticsTrafficParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AnalyticsPagesParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AnalyticsConversionsParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BookingCallParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AiReferralParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "Start date in YYYY-MM-DD format")]
    pub date_from: String,
    #[schemars(description = "End date in YYYY-MM-DD format")]
    pub date_to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AnalyticsCustomParams {
    #[schemars(description = "Account name as configured in herald.toml")]
    pub account: String,
    #[schemars(description = "GA4 Data API runReport request body as JSON string")]
    pub report_json: String,
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

    async fn resolve_ga4_property(&self, name: &str) -> Result<String, McpError> {
        let api = self.api.read().await;
        api.accounts
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .and_then(|a| a.ga4_property_id.clone())
            .ok_or_else(|| {
                McpError::invalid_params(
                    format!("No GA4 property configured for account '{name}'. Add ga4_property_id to herald.toml."),
                    None,
                )
            })
    }

    async fn run_ga4_report(&self, account: &str, body: serde_json::Value) -> Result<serde_json::Value, McpError> {
        let property_id = self.resolve_ga4_property(account).await?;
        let token = self.get_token().await?;
        let api = self.api.read().await;
        google_analytics::run_report(&api.http, &token, &property_id, body)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))
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
            .map(|a| {
                let ga4 = a.ga4_property_id.as_deref().unwrap_or("not configured");
                format!("- {} (Ads: {}, GA4: {})", a.name, a.customer_id, ga4)
            })
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

    // -- Google Analytics 4 tools --

    #[tool(description = "Get GA4 traffic overview by channel for a date range. Returns sessions, users, new users, bounce rate, avg session duration, page views, and conversions broken down by channel (organic, paid, direct, referral, etc).")]
    async fn get_analytics_traffic(
        &self,
        Parameters(AnalyticsTrafficParams { account, date_from, date_to }): Parameters<AnalyticsTrafficParams>,
    ) -> Result<CallToolResult, McpError> {
        let body = google_analytics::traffic_report(&date_from, &date_to);
        let data = self.run_ga4_report(&account, body).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get GA4 top pages report for a date range. Returns page views, users, avg session duration, bounce rate, and conversions per page path.")]
    async fn get_analytics_pages(
        &self,
        Parameters(AnalyticsPagesParams { account, date_from, date_to }): Parameters<AnalyticsPagesParams>,
    ) -> Result<CallToolResult, McpError> {
        let body = google_analytics::pages_report(&date_from, &date_to);
        let data = self.run_ga4_report(&account, body).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get GA4 conversions report for a date range. Returns conversion events broken down by channel group.")]
    async fn get_analytics_conversions(
        &self,
        Parameters(AnalyticsConversionsParams { account, date_from, date_to }): Parameters<AnalyticsConversionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let body = google_analytics::conversions_report(&date_from, &date_to);
        let data = self.run_ga4_report(&account, body).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get booking and call engagement events broken down by channel (organic, paid, direct, referral, etc). Returns booking_click (clicked onto booking form), call_click (clicked phone number), service_selected (chose a service), and service_booked (confirmed real booking) with event count and unique users per channel. Use this to compare lead generation across traffic sources.")]
    async fn get_booking_call_events(
        &self,
        Parameters(BookingCallParams { account, date_from, date_to }): Parameters<BookingCallParams>,
    ) -> Result<CallToolResult, McpError> {
        let body = google_analytics::booking_call_report(&date_from, &date_to);
        let data = self.run_ga4_report(&account, body).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Get traffic and events from AI referral sources (ChatGPT, Copilot, Perplexity, Claude, Gemini). Shows sessions, page views, booking clicks, call clicks, and all other events from AI-driven traffic. Use this to track how much business AI search is sending.")]
    async fn get_ai_referral_traffic(
        &self,
        Parameters(AiReferralParams { account, date_from, date_to }): Parameters<AiReferralParams>,
    ) -> Result<CallToolResult, McpError> {
        let body = google_analytics::ai_referral_report(&date_from, &date_to);
        let data = self.run_ga4_report(&account, body).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Run a custom GA4 Data API report. Pass a JSON string matching the GA4 runReport request body format. Use this for custom analytics queries.")]
    async fn run_analytics_report(
        &self,
        Parameters(AnalyticsCustomParams { account, report_json }): Parameters<AnalyticsCustomParams>,
    ) -> Result<CallToolResult, McpError> {
        let body: serde_json::Value = serde_json::from_str(&report_json)
            .map_err(|e| McpError::invalid_params(format!("Invalid JSON: {e}"), None))?;
        let data = self.run_ga4_report(&account, body).await?;
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
                 Pulls Google Ads and Google Analytics data for analysis across multiple accounts. \
                 Ads tools: list_changes, get_performance, get_keywords, get_search_terms, run_query. \
                 Analytics tools: get_analytics_traffic, get_analytics_pages, get_analytics_conversions, get_booking_call_events, get_ai_referral_traffic, run_analytics_report. \
                 Note: booking_click = clicked onto booking form, service_booked = confirmed real booking. \
                 Use list_accounts first to see available accounts, then pass the account name to other tools."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

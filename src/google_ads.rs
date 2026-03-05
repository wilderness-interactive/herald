use crate::config::AdsConfig;

const ADS_API_VERSION: &str = "v19";

#[derive(Debug)]
pub enum AdsError {
    RequestFailed(String),
    ApiError(String),
}

impl std::fmt::Display for AdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdsError::RequestFailed(msg) => write!(f, "Ads API request failed: {msg}"),
            AdsError::ApiError(msg) => write!(f, "Ads API error: {msg}"),
        }
    }
}

fn ads_url(customer_id: &str) -> String {
    let clean_id = customer_id.replace('-', "");
    format!(
        "https://googleads.googleapis.com/{ADS_API_VERSION}/customers/{clean_id}/googleAds:searchStream"
    )
}

pub async fn query(
    client: &reqwest::Client,
    config: &AdsConfig,
    access_token: &str,
    gaql: &str,
) -> Result<serde_json::Value, AdsError> {
    let url = ads_url(&config.customer_id);

    let mut request = client
        .post(&url)
        .bearer_auth(access_token)
        .header("developer-token", &config.developer_token)
        .json(&serde_json::json!({ "query": gaql }));

    if let Some(login_id) = &config.login_customer_id {
        request = request.header("login-customer-id", login_id.replace('-', ""));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AdsError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AdsError::ApiError(format!("{status}: {body}")));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AdsError::RequestFailed(e.to_string()))?;

    Ok(data)
}

// -- GAQL query builders --
// Pure functions returning query strings — no state, just data transforms

pub fn change_history_query(days_back: u32) -> String {
    format!(
        "SELECT \
            change_event.change_date_time, \
            change_event.change_resource_type, \
            change_event.changed_fields, \
            change_event.client_type, \
            change_event.user_email, \
            change_event.old_resource, \
            change_event.new_resource, \
            campaign.name, \
            ad_group.name \
        FROM change_event \
        WHERE change_event.change_date_time DURING LAST_{days_back}_DAYS \
        ORDER BY change_event.change_date_time DESC \
        LIMIT 100"
    )
}

pub fn campaign_performance_query(date_from: &str, date_to: &str) -> String {
    format!(
        "SELECT \
            campaign.name, \
            campaign.status, \
            metrics.impressions, \
            metrics.clicks, \
            metrics.ctr, \
            metrics.average_cpc, \
            metrics.conversions, \
            metrics.cost_micros, \
            metrics.conversions_value \
        FROM campaign \
        WHERE segments.date BETWEEN '{date_from}' AND '{date_to}' \
        ORDER BY metrics.impressions DESC"
    )
}

pub fn keyword_performance_query(date_from: &str, date_to: &str) -> String {
    format!(
        "SELECT \
            ad_group.name, \
            ad_group_criterion.keyword.text, \
            ad_group_criterion.keyword.match_type, \
            ad_group_criterion.quality_info.quality_score, \
            metrics.impressions, \
            metrics.clicks, \
            metrics.ctr, \
            metrics.average_cpc, \
            metrics.conversions, \
            metrics.cost_micros \
        FROM keyword_view \
        WHERE segments.date BETWEEN '{date_from}' AND '{date_to}' \
        ORDER BY metrics.impressions DESC \
        LIMIT 200"
    )
}

pub fn search_terms_query(date_from: &str, date_to: &str) -> String {
    format!(
        "SELECT \
            search_term_view.search_term, \
            campaign.name, \
            ad_group.name, \
            metrics.impressions, \
            metrics.clicks, \
            metrics.ctr, \
            metrics.conversions, \
            metrics.cost_micros \
        FROM search_term_view \
        WHERE segments.date BETWEEN '{date_from}' AND '{date_to}' \
        ORDER BY metrics.impressions DESC \
        LIMIT 200"
    )
}

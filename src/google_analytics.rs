const GA4_API_URL: &str = "https://analyticsdata.googleapis.com/v1beta";

#[derive(Debug)]
pub enum AnalyticsError {
    RequestFailed(String),
    ApiError(String),
}

impl std::fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyticsError::RequestFailed(msg) => write!(f, "Analytics API request failed: {msg}"),
            AnalyticsError::ApiError(msg) => write!(f, "Analytics API error: {msg}"),
        }
    }
}

pub async fn run_report(
    client: &reqwest::Client,
    access_token: &str,
    property_id: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, AnalyticsError> {
    let url = format!("{GA4_API_URL}/properties/{property_id}:runReport");

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| AnalyticsError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AnalyticsError::ApiError(format!("{status}: {body}")));
    }

    response
        .json()
        .await
        .map_err(|e| AnalyticsError::RequestFailed(e.to_string()))
}

// -- Report builders --
// Pure functions returning JSON request bodies

pub fn traffic_report(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::json!({
        "dateRanges": [{ "startDate": date_from, "endDate": date_to }],
        "dimensions": [
            { "name": "sessionDefaultChannelGroup" }
        ],
        "metrics": [
            { "name": "sessions" },
            { "name": "totalUsers" },
            { "name": "newUsers" },
            { "name": "bounceRate" },
            { "name": "averageSessionDuration" },
            { "name": "screenPageViews" },
            { "name": "conversions" }
        ]
    })
}

pub fn pages_report(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::json!({
        "dateRanges": [{ "startDate": date_from, "endDate": date_to }],
        "dimensions": [
            { "name": "pagePath" }
        ],
        "metrics": [
            { "name": "screenPageViews" },
            { "name": "totalUsers" },
            { "name": "averageSessionDuration" },
            { "name": "bounceRate" },
            { "name": "conversions" }
        ],
        "orderBys": [{ "metric": { "metricName": "screenPageViews" }, "desc": true }],
        "limit": 50
    })
}

pub fn conversions_report(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::json!({
        "dateRanges": [{ "startDate": date_from, "endDate": date_to }],
        "dimensions": [
            { "name": "eventName" },
            { "name": "sessionDefaultChannelGroup" }
        ],
        "metrics": [
            { "name": "eventCount" },
            { "name": "totalUsers" },
            { "name": "conversions" }
        ],
        "dimensionFilter": {
            "filter": {
                "fieldName": "eventName",
                "stringFilter": {
                    "matchType": "EXACT",
                    "value": "conversion"
                }
            }
        }
    })
}

pub fn booking_call_report(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::json!({
        "dateRanges": [{ "startDate": date_from, "endDate": date_to }],
        "dimensions": [
            { "name": "eventName" },
            { "name": "sessionDefaultChannelGroup" }
        ],
        "metrics": [
            { "name": "eventCount" },
            { "name": "totalUsers" }
        ],
        "dimensionFilter": {
            "orGroup": {
                "expressions": [
                    { "filter": { "fieldName": "eventName", "stringFilter": { "matchType": "EXACT", "value": "booking_click" }}},
                    { "filter": { "fieldName": "eventName", "stringFilter": { "matchType": "EXACT", "value": "call_click" }}},
                    { "filter": { "fieldName": "eventName", "stringFilter": { "matchType": "EXACT", "value": "service_selected" }}},
                    { "filter": { "fieldName": "eventName", "stringFilter": { "matchType": "EXACT", "value": "service_booked" }}}
                ]
            }
        },
        "orderBys": [
            { "dimension": { "dimensionName": "eventName" }},
            { "metric": { "metricName": "eventCount" }, "desc": true }
        ]
    })
}

pub fn ai_referral_report(date_from: &str, date_to: &str) -> serde_json::Value {
    serde_json::json!({
        "dateRanges": [{ "startDate": date_from, "endDate": date_to }],
        "dimensions": [
            { "name": "sessionSource" },
            { "name": "eventName" }
        ],
        "metrics": [
            { "name": "eventCount" },
            { "name": "totalUsers" }
        ],
        "dimensionFilter": {
            "orGroup": {
                "expressions": [
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "chatgpt" }}},
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "openai" }}},
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "copilot" }}},
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "perplexity" }}},
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "claude" }}},
                    { "filter": { "fieldName": "sessionSource", "stringFilter": { "matchType": "CONTAINS", "value": "gemini" }}}
                ]
            }
        },
        "orderBys": [
            { "dimension": { "dimensionName": "sessionSource" }},
            { "metric": { "metricName": "eventCount" }, "desc": true }
        ]
    })
}

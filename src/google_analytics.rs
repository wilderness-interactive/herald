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

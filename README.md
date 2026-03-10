# Herald

Sovereign marketing analytics. Claude meets your data.

Herald is a Rust MCP server that bridges Claude with Google Ads, Google Analytics 4, and Atrium CRM. Closed-loop attribution from ad click to actual appointment. No third-party analytics wrappers, no dashboard middleware.

**[wildernessinteractive.com](https://wildernessinteractive.com)**

## Architecture

```
Claude Code <--stdio/MCP--> herald.exe <--REST API--> Google Ads
                                       <--REST API--> Google Analytics 4
                                       <--SQLite----> Atrium CRM
```

- **MCP layer**: rmcp over stdio
- **Google APIs**: Direct REST with OAuth2 token refresh
- **Atrium**: Read-only SQLite for ground-truth attribution

## Tools

### Google Ads

- `list_accounts` - List configured ad accounts
- `list_changes` - Recent account modifications with before/after values
- `get_performance` - Campaign metrics (impressions, clicks, CPC, conversions, cost)
- `get_keywords` - Keyword performance with quality scores
- `get_search_terms` - Actual search queries that triggered ads
- `run_query` - Custom GAQL queries

### Google Analytics 4

- `get_analytics_traffic` - Traffic by channel
- `get_analytics_pages` - Top pages with views and bounce rate
- `get_analytics_conversions` - Conversions by channel
- `get_booking_call_events` - Booking and call engagement events
- `get_ai_referral_traffic` - Traffic from AI sources (ChatGPT, Copilot, Claude, Gemini, Perplexity)
- `run_analytics_report` - Custom GA4 Data API reports

### Atrium CRM

- `get_patient_attribution` - Real appointments with full source attribution
- `get_channel_breakdown` - Bookings by channel with revenue
- `get_lead_pipeline` - Leads by stage (new, contacted, booked, complete)

## Setup

### Build

```
cargo build
```

### Authenticate

```
herald.exe auth
```

Runs the OAuth2 flow for Google API access.

### Configure

Create `herald.toml` with Google OAuth credentials, Ads developer token, and account mappings (Ads customer ID, GA4 property ID, optional Atrium DB path per account).

### Connect to Claude Code

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "herald": {
      "type": "stdio",
      "command": "path/to/herald.exe"
    }
  }
}
```

## License

Wilderness Interactive Open License

Permission is hereby granted, free of charge, to use, copy, modify, and distribute this software for any purpose, including commercial use.

This software may NOT be:
- Sold as a standalone product
- Sold access to as a hosted service

Use for building software, building websites, automating workflows, and integrating with other tools (including commercial work) is explicitly permitted and encouraged. This software is designed to be moddable, so modifications are explicitly permitted and encouraged. Software and systems built using this tool can be sold freely.

The purpose of this license is to prevent reselling the software itself.

---

Built by [Wilderness Interactive](https://wildernessinteractive.com).
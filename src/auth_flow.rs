use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::{self, GoogleConfig};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/adwords";

pub async fn run(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = config::load_config(config_path)?;

    // Bind to any available port on loopback
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let encoded_redirect = urlencod(&redirect_uri);
    let encoded_scope = urlencod(SCOPE);

    let mut auth_url = format!(
        "{AUTH_URL}?client_id={}&redirect_uri={encoded_redirect}&response_type=code&scope={encoded_scope}&access_type=offline&prompt=consent",
        config.google.client_id,
    );

    if let Some(email) = &config.google.email {
        auth_url.push_str(&format!("&login_hint={email}"));
    }

    eprintln!("\nOpening browser for Google sign-in...\n");
    eprintln!("If the browser doesn't open, go to:\n{auth_url}\n");

    // Try to open browser
    let _ = open_browser(&auth_url);

    // Wait for the redirect
    eprintln!("Waiting for authorization...");
    let code = wait_for_code(listener).await?;
    eprintln!("Got authorization code. Exchanging for tokens...");

    // Exchange code for tokens
    let refresh_token = exchange_code(
        &config.google,
        &code,
        &redirect_uri,
    ).await?;

    // Save to config
    config.google.refresh_token = Some(refresh_token);
    config::save_config(config_path, &config)?;

    eprintln!("\nDone! refresh_token saved to {config_path}");
    eprintln!("Herald is ready to use.");

    Ok(())
}

async fn wait_for_code(listener: TcpListener) -> Result<String, Box<dyn std::error::Error>> {
    let (mut stream, _) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the code from "GET /?code=XXXX&scope=... HTTP/1.1"
    let code = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| {
            path.split('?')
                .nth(1)?
                .split('&')
                .find(|p| p.starts_with("code="))
                .map(|p| p.strip_prefix("code=").unwrap_or("").to_owned())
        })
        .ok_or("No authorization code in redirect")?;

    // Check for error
    if code.is_empty() {
        return Err("Empty authorization code".into());
    }

    // Send a nice response to the browser
    let body = "<!DOCTYPE html><html><body><h2>Herald authorized!</h2><p>You can close this tab.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;

    Ok(code)
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    refresh_token: Option<String>,
}

async fn exchange_code(
    google: &GoogleConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", google.client_id.as_str()),
            ("client_secret", google.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {status}: {body}").into());
    }

    let token_data: TokenResponse = response.json().await?;

    token_data
        .refresh_token
        .ok_or_else(|| "No refresh_token in response. Try adding prompt=consent to force a new one.".into())
}

fn urlencod(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

fn open_browser(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

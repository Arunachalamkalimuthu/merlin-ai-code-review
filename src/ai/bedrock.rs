//! Amazon Bedrock provider — Claude models via AWS Bedrock Runtime.
//!
//! Uses the Anthropic Messages API format wrapped for Bedrock,
//! with AWS Signature Version 4 (SigV4) request signing.
//!
//! Requires env vars: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`
//! Optional: `AWS_SESSION_TOKEN` (for assumed roles / STS)
//!
//! Config:
//! ```toml
//! [ai]
//! provider = "bedrock"
//! model = "anthropic.claude-sonnet-4-6-20250514-v1:0"
//! bedrock_region = "us-east-1"
//! ```

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, instrument};

use super::{system_prompt, AiProvider, ReviewComment, ReviewContext};
use crate::config::AiConfig;
use crate::error::{MerlinError, Result};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_REGION: &str = "us-east-1";
const SERVICE: &str = "bedrock";
const BEDROCK_SERVICE: &str = "bedrock-runtime";

/// AI provider for Amazon Bedrock (Claude models via SigV4 auth).
pub struct BedrockProvider {
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
    config: AiConfig,
    client: reqwest::Client,
}

impl BedrockProvider {
    /// Create a new Bedrock provider from credentials and config.
    pub fn new(
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        config: AiConfig,
    ) -> Self {
        Self {
            access_key,
            secret_key,
            session_token,
            config,
            client: reqwest::Client::new(),
        }
    }

    fn region(&self) -> &str {
        self.config
            .bedrock_region
            .as_deref()
            .unwrap_or(DEFAULT_REGION)
    }

    fn endpoint(&self) -> String {
        format!(
            "https://{SERVICE}-runtime.{region}.amazonaws.com/model/{model}/invoke",
            region = self.region(),
            model = self.config.model,
        )
    }

    /// Sign the request using AWS Signature Version 4.
    fn sign_request(
        &self,
        method: &str,
        url: &str,
        payload: &str,
        datetime: &str,
        date: &str,
    ) -> Result<String> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| MerlinError::Config(format!("Invalid Bedrock URL: {e}")))?;
        let host = parsed.host_str().unwrap_or_default();
        let path = parsed.path();

        // Step 1: Canonical request
        let payload_hash = hex::encode(Sha256::digest(payload.as_bytes()));
        let canonical_headers =
            format!("content-type:application/json\nhost:{host}\nx-amz-date:{datetime}\n");
        let signed_headers = "content-type;host;x-amz-date";
        let canonical_request =
            format!("{method}\n{path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");

        // Step 2: String to sign
        let region = self.region();
        let credential_scope = format!("{date}/{region}/{BEDROCK_SERVICE}/aws4_request");
        let request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{datetime}\n{credential_scope}\n{request_hash}");

        // Step 3: Derive signing key
        let signing_key = self.derive_signing_key(date, region)?;

        // Step 4: Signature
        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|e| MerlinError::Config(format!("HMAC error: {e}")))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // Step 5: Authorization header
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{credential_scope},SignedHeaders={signed_headers},Signature={signature}",
            self.access_key
        ))
    }

    fn derive_signing_key(&self, date: &str, region: &str) -> Result<Vec<u8>> {
        let k_secret = format!("AWS4{}", self.secret_key);

        let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes())?;
        let k_region = hmac_sha256(&k_date, region.as_bytes())?;
        let k_service = hmac_sha256(&k_region, BEDROCK_SERVICE.as_bytes())?;
        let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
        Ok(k_signing)
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| MerlinError::Config(format!("HMAC key error: {e}")))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn utc_now() -> (String, String) {
    // Format: datetime = "20240101T120000Z", date = "20240101"
    // Use std time to avoid adding chrono dep
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple UTC formatting from unix timestamp
    let s = format_utc(secs);
    let date = s[..8].to_string();
    (s, date)
}

fn format_utc(secs: u64) -> String {
    // Days since epoch
    let mut remaining = secs;
    let seconds = remaining % 60;
    remaining /= 60;
    let minutes = remaining % 60;
    remaining /= 60;
    let hours = remaining % 24;
    remaining /= 24;

    let mut days = remaining as u32;
    let mut year = 1970u32;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u32; 12] = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    while month < 11 && days >= month_days[month] {
        days -= month_days[month];
        month += 1;
    }
    let day = days + 1;

    format!(
        "{year:04}{month:02}{day:02}T{hours:02}{minutes:02}{seconds:02}Z",
        month = month + 1
    )
}

fn is_leap(year: u32) -> bool {
    year % 400 == 0 || (year % 4 == 0 && year % 100 != 0)
}

// ── Bedrock request/response types ───────────────────────────────────────────

#[derive(Serialize)]
struct BedrockRequest {
    anthropic_version: &'static str,
    max_tokens: u32,
    system: String,
    messages: Vec<BedrockMessage>,
}

#[derive(Serialize)]
struct BedrockMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct BedrockResponse {
    content: Vec<BedrockContent>,
}

#[derive(Deserialize)]
struct BedrockContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl AiProvider for BedrockProvider {
    #[instrument(skip(self, system, user))]
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = self.endpoint();
        let request = BedrockRequest {
            anthropic_version: "bedrock-2023-05-31",
            max_tokens: self.config.max_tokens,
            system: system.to_string(),
            messages: vec![BedrockMessage {
                role: "user",
                content: user.to_string(),
            }],
        };

        let payload = serde_json::to_string(&request).map_err(MerlinError::Json)?;

        let (datetime, date) = utc_now();
        let auth_header = self.sign_request("POST", &url, &payload, &datetime, &date)?;

        debug!("Sending request to Bedrock model: {}", self.config.model);

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .header("x-amz-date", &datetime)
            .header("Authorization", auth_header);

        if let Some(ref token) = self.session_token {
            req = req.header("x-amz-security-token", token);
        }

        let resp = req.body(payload).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MerlinError::AiProvider(format!(
                "Bedrock error {status}: {body}"
            )));
        }

        let result: BedrockResponse = resp.json().await?;
        result
            .content
            .into_iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text)
            .ok_or_else(|| MerlinError::AiProvider("Empty Bedrock response".to_string()))
    }

    #[instrument(skip(self, ctx), fields(file = %ctx.file))]
    async fn review(&self, ctx: &ReviewContext) -> Result<Vec<ReviewComment>> {
        let system = system_prompt(&[
            "bugs".to_string(),
            "security".to_string(),
            "style".to_string(),
            "performance".to_string(),
        ]);
        let user = format!(
            "Review the following diff for file `{}`:\n\n```diff\n{}\n```",
            ctx.file, ctx.diff_hunk
        );
        let raw = self.generate(&system, &user).await?;
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        serde_json::from_str(cleaned).map_err(|e| {
            MerlinError::AiProvider(format!(
                "Failed to parse Bedrock response: {e}\nRaw: {cleaned}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_utc() {
        // 2024-01-01 00:00:00 UTC = 1704067200 seconds since epoch
        let s = format_utc(1_704_067_200);
        assert_eq!(s, "20240101T000000Z");
    }

    #[test]
    fn test_is_leap() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
    }
}

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;

use crate::Language;

use super::{AnyProvider, ensure_provider_success, resolve_system_prompt};

pub struct ClaudeProvider {
    pub http_client: Arc<ClientWithMiddleware>,
    pub api_key: String,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct UserMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: String,
    messages: Vec<UserMessage>,
}

fn serialize_request(
    model: &str,
    max_tokens: u32,
    system: &str,
    source: &str,
) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&MessagesRequest {
        model,
        max_tokens,
        system: system.to_string(),
        messages: vec![UserMessage {
            role: "user",
            content: source.to_string(),
        }],
    })
    .map_err(Into::into)
}

impl AnyProvider for ClaudeProvider {
    fn translate<'a>(
        &'a self,
        source: &'a str,
        target_language: Language,
        model: &'a str,
        custom_system_prompt: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let system = resolve_system_prompt(custom_system_prompt, target_language);
            let body = serialize_request(model, self.max_tokens.unwrap_or(8192), &system, source)?;

            let response = self
                .http_client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .body(body)
                .send()
                .await?;

            let resp: serde_json::Value = ensure_provider_success("claude", response)
                .await?
                .json()
                .await?;

            let text = resp["content"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Claude returned no content"))?
                .to_string();

            Ok(text)
        })
    }

    fn generate<'a>(
        &'a self,
        source: &'a str,
        _target_language: Language,
        model: &'a str,
        system_prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let body = serialize_request(
                model,
                self.max_tokens.unwrap_or(8192),
                system_prompt,
                source,
            )?;
            let response = self
                .http_client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .body(body)
                .send()
                .await?;
            let resp: serde_json::Value = ensure_provider_success("claude", response)
                .await?
                .json()
                .await?;
            resp["content"][0]["text"]
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("Claude returned no content"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_configured_max_tokens() {
        let body: serde_json::Value = serde_json::from_slice(
            &serialize_request("claude-test", 1357, "system", "source").unwrap(),
        )
        .unwrap();
        assert_eq!(body["max_tokens"], 1357);
    }
}

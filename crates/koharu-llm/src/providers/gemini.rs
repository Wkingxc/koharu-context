use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;

use crate::Language;

use super::{AnyProvider, ensure_provider_success, resolve_system_prompt};

pub struct GeminiProvider {
    pub http_client: Arc<ClientWithMiddleware>,
    pub api_key: String,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    system_instruction: SystemInstruction,
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    max_output_tokens: u32,
}

fn serialize_request(
    system: &str,
    source: &str,
    max_tokens: Option<u32>,
) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&GenerateRequest {
        system_instruction: SystemInstruction {
            parts: vec![Part {
                text: system.to_string(),
            }],
        },
        contents: vec![Content {
            parts: vec![Part {
                text: source.to_string(),
            }],
        }],
        generation_config: max_tokens
            .map(|max_output_tokens| GenerationConfig { max_output_tokens }),
    })
    .map_err(Into::into)
}

impl AnyProvider for GeminiProvider {
    fn translate<'a>(
        &'a self,
        source: &'a str,
        target_language: Language,
        model: &'a str,
        custom_system_prompt: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            );

            let system = resolve_system_prompt(custom_system_prompt, target_language);
            let body = serialize_request(&system, source, self.max_tokens)?;

            let response = self
                .http_client
                .post(&url)
                .header("content-type", "application/json")
                .body(body)
                .send()
                .await?;

            let resp: serde_json::Value = ensure_provider_success("gemini", response)
                .await?
                .json()
                .await?;

            let text = resp["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Gemini returned no content"))?
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
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model, self.api_key
            );
            let response = self
                .http_client
                .post(&url)
                .header("content-type", "application/json")
                .body(serialize_request(system_prompt, source, self.max_tokens)?)
                .send()
                .await?;
            let resp: serde_json::Value = ensure_provider_success("gemini", response)
                .await?
                .json()
                .await?;
            resp["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("Gemini returned no content"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_configured_max_output_tokens() {
        let body: serde_json::Value =
            serde_json::from_slice(&serialize_request("system", "source", Some(9753)).unwrap())
                .unwrap();
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 9753);
    }
}

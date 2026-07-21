use std::sync::Arc;

use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;

use super::ensure_provider_success;

pub enum ChatCompletionsAuth {
    None,
    Bearer(String),
}

pub struct ChatCompletionsRequest {
    pub provider: &'static str,
    pub endpoint: String,
    pub auth: ChatCompletionsAuth,
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

fn serialize_request_body(request: &ChatCompletionsRequest) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&ChatRequest {
        model: &request.model,
        messages: vec![
            ChatMessage {
                role: "system",
                content: request.system_prompt.clone(),
            },
            ChatMessage {
                role: "user",
                content: request.user_prompt.clone(),
            },
        ],
        temperature: request.temperature,
        max_tokens: request.max_tokens,
    })
    .map_err(Into::into)
}

pub async fn send_chat_completion(
    http_client: Arc<ClientWithMiddleware>,
    request: ChatCompletionsRequest,
) -> anyhow::Result<String> {
    let body = serialize_request_body(&request)?;

    let mut http_request = http_client.post(&request.endpoint);
    if let ChatCompletionsAuth::Bearer(api_key) = request.auth {
        http_request = http_request.bearer_auth(api_key);
    }

    let response = http_request
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;

    let resp: serde_json::Value = ensure_provider_success(request.provider, response)
        .await?
        .json()
        .await?;

    resp["choices"][0]["message"]["content"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("{} returned no content", request.provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_optional_max_tokens() {
        let request = ChatCompletionsRequest {
            provider: "test",
            endpoint: "https://example.test/chat".to_string(),
            auth: ChatCompletionsAuth::None,
            model: "model".to_string(),
            system_prompt: "system".to_string(),
            user_prompt: "user".to_string(),
            temperature: None,
            max_tokens: Some(1234),
        };

        let body: serde_json::Value =
            serde_json::from_slice(&serialize_request_body(&request).unwrap()).unwrap();
        assert_eq!(body["max_tokens"], 1234);

        let request = ChatCompletionsRequest {
            max_tokens: None,
            ..request
        };
        let body: serde_json::Value =
            serde_json::from_slice(&serialize_request_body(&request).unwrap()).unwrap();
        assert!(body.get("max_tokens").is_none());
    }
}

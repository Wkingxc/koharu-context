use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use reqwest_middleware::ClientWithMiddleware;

use crate::Language;

use super::AnyProvider;
use super::chat_completions::{ChatCompletionsAuth, ChatCompletionsRequest, send_chat_completion};
use super::resolve_system_prompt;

pub struct DeepSeekProvider {
    pub http_client: Arc<ClientWithMiddleware>,
    pub api_key: String,
    pub max_tokens: Option<u32>,
}

fn build_request(
    api_key: &str,
    max_tokens: Option<u32>,
    source: &str,
    target_language: Language,
    model: &str,
    custom_system_prompt: Option<&str>,
) -> ChatCompletionsRequest {
    build_raw_request(
        api_key,
        max_tokens,
        source,
        model,
        resolve_system_prompt(custom_system_prompt, target_language),
    )
}

fn build_raw_request(
    api_key: &str,
    max_tokens: Option<u32>,
    source: &str,
    model: &str,
    system_prompt: String,
) -> ChatCompletionsRequest {
    ChatCompletionsRequest {
        provider: "deepseek",
        endpoint: "https://api.deepseek.com/chat/completions".to_string(),
        auth: ChatCompletionsAuth::Bearer(api_key.to_string()),
        model: model.to_string(),
        system_prompt,
        user_prompt: source.to_string(),
        temperature: Some(1.3),
        max_tokens,
    }
}

impl AnyProvider for DeepSeekProvider {
    fn translate<'a>(
        &'a self,
        source: &'a str,
        target_language: Language,
        model: &'a str,
        custom_system_prompt: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            send_chat_completion(
                Arc::clone(&self.http_client),
                build_request(
                    &self.api_key,
                    self.max_tokens,
                    source,
                    target_language,
                    model,
                    custom_system_prompt,
                ),
            )
            .await
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
            send_chat_completion(
                Arc::clone(&self.http_client),
                build_raw_request(
                    &self.api_key,
                    self.max_tokens,
                    source,
                    model,
                    system_prompt.to_string(),
                ),
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_max_tokens_to_chat_request() {
        let request = build_request(
            "key",
            Some(2468),
            "source",
            Language::English,
            "deepseek-test",
            None,
        );
        assert_eq!(request.max_tokens, Some(2468));
    }
}

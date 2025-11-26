use super::AIProvider;
use async_trait::async_trait;
use openrouter::{
    completions::{request::{Message, Content}, Request},
    OpenRouter,
};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};

/// OpenRouter AI provider implementation
pub struct OpenRouterProvider {
    client: OpenRouter,
    model: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let client = OpenRouter::new(api_key)
            .with_site_url("https://github.com/afadil/wealthfolio")
            .unwrap()
            .with_site_title("Wealthfolio")
            .unwrap();

        Self { client, model }
    }
}

#[async_trait]
impl AIProvider for OpenRouterProvider {
    async fn complete(
        &self,
        prompt: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut request = Request::default();
        request.model = Some(self.model.clone());
        request.messages = Some(vec![Message::User {
            content: Content::Plain(prompt),
            name: None,
            cache_control: None,
        }]);

        let response = self.client.chat_completion(request).await?;

        let content = response
            .choices
            .first()
            .and_then(|c| match c {
                openrouter::completions::response::Choice::NonStreaming(choice) => {
                    choice.message.content.clone()
                }
                _ => None,
            })
            .ok_or("No response from OpenRouter")?;

        Ok(content)
    }

    fn provider_name(&self) -> &str {
        "OpenRouter"
    }
}

/// Ollama AI provider implementation (local)
pub struct OllamaProvider {
    client: Ollama,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String) -> Self {
        // Parse the URL to extract host and port
        let url = base_url.trim_end_matches('/');
        let (host, port) = if let Some(idx) = url.rfind(':') {
            let host = url[..idx].trim_start_matches("http://").trim_start_matches("https://");
            let port = url[idx + 1..].parse().unwrap_or(11434);
            (host.to_string(), port)
        } else {
            let host = url.trim_start_matches("http://").trim_start_matches("https://");
            (host.to_string(), 11434)
        };

        let client = Ollama::new(host, port);
        Self { client, model }
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    async fn complete(&self, prompt: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request = GenerationRequest::new(self.model.clone(), prompt);
        let response = self.client.generate(request).await?;
        Ok(response.response)
    }

    fn provider_name(&self) -> &str {
        "Ollama"
    }
}

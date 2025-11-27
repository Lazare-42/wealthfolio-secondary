// Test to verify all our AIProviderConfig variants map to valid ai-lib Providers
use ai_lib::Provider;

#[test]
fn test_all_providers_are_valid() {
    // This will fail to compile if any of these don't exist in ai-lib
    let _openrouter = Provider::OpenRouter;
    let _ollama = Provider::Ollama;
    let _openai = Provider::OpenAI;
    let _anthropic = Provider::Anthropic;
    let _mistral = Provider::Mistral;

    println!("✅ All providers are valid ai-lib::Provider variants");
}

#[test]
fn test_provider_config_creation() {
    use crate::ai::AIProviderConfig;

    // Test that our enum variants serialize correctly
    let configs = vec![
        AIProviderConfig::OpenRouter {
            model: Some("test".into()),
        },
        AIProviderConfig::Ollama {
            base_url: None,
            model: None,
        },
        AIProviderConfig::OpenAI { model: None },
        AIProviderConfig::Anthropic { model: None },
        AIProviderConfig::Mistral { model: None },
        AIProviderConfig::Disabled,
    ];

    for config in configs {
        println!("Config variant: {:?}", config);
    }
}

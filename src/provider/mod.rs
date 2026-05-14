pub mod traits;
mod zenmux;

pub use traits::*;
pub use zenmux::{ZenmuxGoogleProvider, ZenmuxOpenAiProvider};

use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    ZenmuxOpenAi,
    ZenmuxGoogle,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::ZenmuxOpenAi => "zenmux/openai",
            ProviderType::ZenmuxGoogle => "zenmux/google",
        }
    }

    pub fn env_var(&self) -> &'static str {
        "ZENMUX_API_KEY"
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::ZenmuxOpenAi => "ZenMux OpenAI Images",
            ProviderType::ZenmuxGoogle => "ZenMux Google Gemini / Imagen",
        }
    }

    pub fn protocol(&self) -> &'static str {
        match self {
            ProviderType::ZenmuxOpenAi => "OpenAI Images",
            ProviderType::ZenmuxGoogle => "Google Gemini / Imagen / Vertex AI",
        }
    }
}

impl FromStr for ProviderType {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "zenmux" | "zenmux/openai" | "zenmux-openai" | "openai" | "openai/images" => {
                Ok(ProviderType::ZenmuxOpenAi)
            }
            "zenmux/google" | "zenmux-google" | "zenmux/gemini" | "zenmux-vertex" | "google"
            | "gemini" | "vertex" | "vertex-ai" => Ok(ProviderType::ZenmuxGoogle),
            _ => Err(()),
        }
    }
}

pub fn create_provider(
    provider_type: ProviderType,
    api_key: Option<String>,
) -> Arc<dyn ImageProvider> {
    match provider_type {
        ProviderType::ZenmuxOpenAi => Arc::new(ZenmuxOpenAiProvider::new(api_key)),
        ProviderType::ZenmuxGoogle => Arc::new(ZenmuxGoogleProvider::new(api_key)),
    }
}

pub fn supported_providers() -> Vec<ProviderType> {
    vec![ProviderType::ZenmuxOpenAi, ProviderType::ZenmuxGoogle]
}

#[cfg(test)]
mod tests {
    use super::ProviderType;
    use std::str::FromStr;

    #[test]
    fn parses_provider_aliases() {
        assert_eq!(ProviderType::from_str("openai").unwrap(), ProviderType::ZenmuxOpenAi);
        assert_eq!(ProviderType::from_str("zenmux/openai").unwrap(), ProviderType::ZenmuxOpenAi);
        assert_eq!(ProviderType::from_str("google").unwrap(), ProviderType::ZenmuxGoogle);
        assert_eq!(ProviderType::from_str("zenmux/gemini").unwrap(), ProviderType::ZenmuxGoogle);
        assert!(ProviderType::from_str("unknown").is_err());
    }
}

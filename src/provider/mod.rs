pub mod traits;
mod zenmux;

pub use traits::*;
pub use zenmux::ZenmuxProvider;

use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    Zenmux,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Zenmux => "zenmux",
        }
    }

    pub fn env_var(&self) -> &'static str {
        "ZENMUX_API_KEY"
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::Zenmux => "ZenMux",
        }
    }

    pub fn protocol(&self) -> &'static str {
        match self {
            ProviderType::Zenmux => "OpenAI Images / Google Gemini / Google Imagen",
        }
    }
}

impl FromStr for ProviderType {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "zenmux" => Ok(ProviderType::Zenmux),
            _ => Err(()),
        }
    }
}

pub fn create_provider(
    provider_type: ProviderType,
    api_key: Option<String>,
) -> Arc<dyn ImageProvider> {
    match provider_type {
        ProviderType::Zenmux => Arc::new(ZenmuxProvider::new(api_key)),
    }
}

pub fn supported_providers() -> Vec<ProviderType> {
    vec![ProviderType::Zenmux]
}

#[cfg(test)]
mod tests {
    use super::ProviderType;
    use std::str::FromStr;

    #[test]
    fn parses_zenmux_provider() {
        assert_eq!(ProviderType::from_str("zenmux").unwrap(), ProviderType::Zenmux);
        assert!(ProviderType::from_str("zenmux/openai").is_err());
        assert!(ProviderType::from_str("zenmux/google").is_err());
        assert!(ProviderType::from_str("openai").is_err());
        assert!(ProviderType::from_str("google").is_err());
        assert!(ProviderType::from_str("unknown").is_err());
    }
}

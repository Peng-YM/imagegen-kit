#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        if std::env::var("DEBUG").map(|value| value == "1").unwrap_or(false) {
            eprintln!($($arg)*);
        }
    };
}

pub mod auth;
pub mod cache;
pub mod error;
pub mod provider;
pub mod utils;

pub use auth::{delete_credential, get_credential, list_credentials, provider_key, set_credential};
pub use cache::{CacheEntry, CacheManager, CACHE_DISABLE_ENV_VAR};
pub use error::{anyhow, Result};
pub use provider::{
    create_provider, EditRequest, GenerateRequest, ImageArtifact, ImageProvider, ImageResult,
    ProgressUpdate, ProviderType,
};

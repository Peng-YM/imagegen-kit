use crate::error::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const MODELS_JSON: &str = include_str!("../models.json");

static CATALOG: OnceLock<ModelCatalog> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelCatalog {
    pub version: u32,
    pub updated_at: String,
    pub source_urls: Vec<String>,
    pub defaults: HashMap<String, String>,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api_model: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub protocols: Vec<String>,
    #[serde(default)]
    pub source_protocols: Vec<String>,
    #[serde(default)]
    pub google_method: Option<GoogleMethod>,
    pub supports_generate: bool,
    pub supports_edit: bool,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum GoogleMethod {
    #[serde(rename = "generateContent")]
    GenerateContent,
    #[serde(rename = "predict")]
    Predict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelOperation {
    Generate,
    Edit,
}

pub fn catalog() -> &'static ModelCatalog {
    CATALOG.get_or_init(|| {
        serde_json::from_str(MODELS_JSON).expect("embedded models.json must be valid JSON")
    })
}

pub fn models_for_provider(provider_id: &str) -> Vec<&'static ModelEntry> {
    catalog().models.iter().filter(|model| model.provider == provider_id).collect()
}

pub fn supported_model_ids(provider_id: &str) -> Vec<String> {
    let mut ids = models_for_provider(provider_id)
        .into_iter()
        .map(|model| model.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

pub fn default_model(provider_id: &str) -> Result<&'static ModelEntry> {
    let default_id = catalog()
        .defaults
        .get(provider_id)
        .ok_or_else(|| anyhow!("No default model configured for {}", provider_id))?;

    catalog()
        .models
        .iter()
        .find(|model| model.provider == provider_id && model_matches(model, default_id))
        .ok_or_else(|| {
            anyhow!(
                "Default model '{}' for {} is missing from embedded models.json",
                default_id,
                provider_id
            )
        })
}

pub fn resolve_model(
    provider_id: &str,
    requested_model: Option<&str>,
    operation: ModelOperation,
) -> Result<ModelEntry> {
    let model = match requested_model {
        Some(requested_model) => find_model(requested_model).ok_or_else(|| {
            anyhow!(
                "Unknown model '{}'. Supported models for {}: {}",
                requested_model,
                provider_id,
                supported_model_ids(provider_id).join(", ")
            )
        })?,
        None => default_model(provider_id)?,
    };

    if model.provider != provider_id {
        return Err(anyhow!(
            "Model '{}' is configured for {}; use --provider {}.",
            model.id,
            model.provider,
            model.provider
        ));
    }

    if !supports_operation(model, operation) {
        return Err(anyhow!(
            "Model '{}' does not support image {} through {}.",
            model.id,
            operation_name(operation),
            provider_id
        ));
    }

    Ok(model.clone())
}

fn find_model(requested_model: &str) -> Option<&'static ModelEntry> {
    catalog().models.iter().find(|model| model_matches(model, requested_model))
}

fn model_matches(model: &ModelEntry, requested_model: &str) -> bool {
    model.id == requested_model
        || model.api_model == requested_model
        || model.aliases.iter().any(|alias| alias == requested_model)
}

fn supports_operation(model: &ModelEntry, operation: ModelOperation) -> bool {
    match operation {
        ModelOperation::Generate => model.supports_generate,
        ModelOperation::Edit => model.supports_edit,
    }
}

fn operation_name(operation: ModelOperation) -> &'static str {
    match operation {
        ModelOperation::Generate => "generation",
        ModelOperation::Edit => "editing",
    }
}

#[cfg(test)]
mod tests {
    use super::{catalog, resolve_model, GoogleMethod, ModelOperation};

    #[test]
    fn loads_embedded_catalog() {
        let catalog = catalog();
        assert_eq!(catalog.version, 1);
        assert!(catalog.models.iter().any(|model| model.id == "openai/gpt-image-2"));
        assert!(catalog.models.iter().any(|model| model.id == "qwen/qwen-image-2.0"));
    }

    #[test]
    fn resolves_openai_alias_to_api_model() {
        let model =
            resolve_model("zenmux/openai", Some("gpt-image-2"), ModelOperation::Generate).unwrap();
        assert_eq!(model.id, "openai/gpt-image-2");
        assert_eq!(model.api_model, "gpt-image-2");
    }

    #[test]
    fn keeps_openai_models_out_of_google_provider() {
        let error =
            resolve_model("zenmux/google", Some("openai/gpt-image-2"), ModelOperation::Generate)
                .unwrap_err();

        assert!(error.to_string().contains("zenmux/openai"));
    }

    #[test]
    fn resolves_non_openai_imagen_models_for_google_provider() {
        let model =
            resolve_model("zenmux/google", Some("qwen/qwen-image-2.0"), ModelOperation::Generate)
                .unwrap();

        assert_eq!(model.google_method, Some(GoogleMethod::Predict));
    }
}

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub message: String,
    pub current: u64,
    pub total: Option<u64>,
}

impl ProgressUpdate {
    pub fn new(message: String) -> Self {
        Self { message, current: 0, total: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: Option<String>,
    pub size: String,
    pub count: u32,
    pub quality: Option<String>,
    pub output_format: Option<String>,
    pub output_compression: Option<u8>,
    pub background: Option<String>,
    pub output_dir: PathBuf,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRequest {
    pub input: PathBuf,
    pub mask: Option<PathBuf>,
    pub prompt: String,
    pub model: Option<String>,
    pub size: String,
    pub quality: Option<String>,
    pub output_format: Option<String>,
    pub output_compression: Option<u8>,
    pub background: Option<String>,
    pub output_dir: PathBuf,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageArtifact {
    pub path: PathBuf,
    pub mime_type: String,
    pub seed: Option<u64>,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    pub provider: String,
    pub model: Option<String>,
    pub artifacts: Vec<ImageArtifact>,
}

#[async_trait::async_trait]
pub trait ImageProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate(
        &self,
        request: GenerateRequest,
        progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult>;

    async fn edit(
        &self,
        request: EditRequest,
        progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult>;
}

use crate::error::{anyhow, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const CACHE_DISABLE_ENV_VAR: &str = "IMAGEGEN_KIT_NO_CACHE";

pub fn is_cache_disabled() -> bool {
    std::env::var(CACHE_DISABLE_ENV_VAR)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub prompt_hash: String,
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
    pub options_hash: String,
    pub output_files: Vec<String>,
    pub created_at: u64,
}

impl CacheEntry {
    pub fn new(
        prompt_hash: String,
        prompt: String,
        provider: String,
        model: Option<String>,
        options_hash: String,
        output_files: Vec<String>,
    ) -> Self {
        Self {
            prompt_hash,
            prompt,
            provider,
            model,
            options_hash,
            output_files,
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

pub struct CacheManager {
    cache_dir: PathBuf,
    index_path: PathBuf,
}

impl CacheManager {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "imagegen-kit")
            .ok_or_else(|| anyhow!("Failed to determine project directories"))?;
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        let index_path = cache_dir.join("index.json");

        fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir, index_path })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    fn load_index(&self) -> Result<HashMap<String, CacheEntry>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.index_path)?;
        let index = serde_json::from_str(&content)?;
        Ok(index)
    }

    fn save_index(&self, index: &HashMap<String, CacheEntry>) -> Result<()> {
        let content = serde_json::to_string_pretty(index)?;
        fs::write(&self.index_path, content)?;
        Ok(())
    }

    pub fn compute_prompt_hash(prompt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn compute_file_hash(path: &Path) -> Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn compute_options_hash(options: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(options.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn generate_cache_key(
        prompt_hash: &str,
        provider: &str,
        model: Option<&str>,
        options_hash: &str,
    ) -> String {
        format!("{}:{}:{}:{}", prompt_hash, provider, model.unwrap_or("default"), options_hash)
    }

    pub fn get(
        &self,
        prompt_hash: &str,
        provider: &str,
        model: Option<&str>,
        options_hash: &str,
    ) -> Result<Option<CacheEntry>> {
        if is_cache_disabled() {
            return Ok(None);
        }

        let index = self.load_index()?;
        let key = Self::generate_cache_key(prompt_hash, provider, model, options_hash);
        Ok(index.get(&key).cloned())
    }

    pub fn put(&self, entry: CacheEntry) -> Result<()> {
        if is_cache_disabled() {
            return Ok(());
        }

        let mut index = self.load_index()?;
        let key = Self::generate_cache_key(
            &entry.prompt_hash,
            &entry.provider,
            entry.model.as_deref(),
            &entry.options_hash,
        );
        index.insert(key, entry);
        self.save_index(&index)
    }

    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
        }
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    pub fn cache_size(&self) -> Result<(usize, u64)> {
        let index = self.load_index()?;
        let size = dir_size(&self.cache_dir)?;
        Ok((index.len(), size))
    }
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    if !path.exists() {
        return Ok(size);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            size += dir_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }

    Ok(size)
}

use crate::error::{anyhow, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

pub fn parse_size(size: &str) -> Result<(u32, u32)> {
    if size.eq_ignore_ascii_case("auto") {
        return Ok((0, 0));
    }

    let (width, height) = size
        .split_once('x')
        .ok_or_else(|| anyhow!("Invalid size '{}'. Expected WIDTHxHEIGHT, e.g. 1024x1024", size))?;
    let width = width.parse::<u32>().map_err(|_| anyhow!("Invalid width in size '{}'", size))?;
    let height = height.parse::<u32>().map_err(|_| anyhow!("Invalid height in size '{}'", size))?;

    if width == 0 || height == 0 {
        return Err(anyhow!("Image size must be greater than zero"));
    }

    Ok((width, height))
}

pub fn default_output_dir() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    for _ in 0..16 {
        let suffix = OsRng.next_u64();
        let path = temp_dir.join(format!("imagegen-kit-{suffix:016x}"));
        if !path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!("Failed to allocate a unique temporary output directory"))
}

pub fn write_json_pretty<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{default_output_dir, is_url, parse_size};

    #[test]
    fn detects_http_urls() {
        assert!(is_url("https://example.com/a.png"));
        assert!(is_url("http://example.com/a.png"));
        assert!(!is_url("./a.png"));
    }

    #[test]
    fn parses_image_sizes() {
        assert_eq!(parse_size("1024x768").unwrap(), (1024, 768));
        assert_eq!(parse_size("auto").unwrap(), (0, 0));
        assert!(parse_size("1024").is_err());
        assert!(parse_size("0x1024").is_err());
    }

    #[test]
    fn default_output_dir_uses_random_temp_path() {
        let temp_dir = std::env::temp_dir();
        let first = default_output_dir().unwrap();
        let second = default_output_dir().unwrap();

        assert_eq!(first.parent(), Some(temp_dir.as_path()));
        assert!(first
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .starts_with("imagegen-kit-"));
        assert_ne!(first, second);
    }
}

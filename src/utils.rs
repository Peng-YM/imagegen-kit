use crate::error::{anyhow, Result};
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
    Ok(std::env::current_dir()?.join("imagegen-output"))
}

pub fn write_json_pretty<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_url, parse_size};

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
}

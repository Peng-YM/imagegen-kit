use super::{
    EditRequest, GenerateRequest, ImageArtifact, ImageProvider, ImageResult, ProgressUpdate,
};
use crate::error::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ZENMUX_OPENAI_BASE_URL: &str = "https://zenmux.ai/api/v1";
const ZENMUX_VERTEX_BASE_URL: &str = "https://zenmux.ai/api/vertex-ai/v1";
const DEFAULT_OPENAI_MODEL: &str = "gpt-image-2";
const DEFAULT_GOOGLE_MODEL: &str = "google/gemini-3-pro-image-preview";

pub struct ZenmuxOpenAiProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl ZenmuxOpenAiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key, client: http_client() }
    }

    fn api_key(&self) -> Result<&str> {
        self.api_key.as_deref().filter(|key| !key.is_empty()).ok_or_else(|| {
            anyhow!("ZENMUX_API_KEY is required. Run imagegen-kit login --provider zenmux/openai")
        })
    }
}

#[async_trait::async_trait]
impl ImageProvider for ZenmuxOpenAiProvider {
    fn name(&self) -> &'static str {
        "ZenMux OpenAI Images"
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        progress_cb(ProgressUpdate::new("Calling ZenMux OpenAI Images API".to_string()));

        let model = request.model.clone().unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string());
        let mut body = json!({
            "model": model,
            "prompt": request.prompt,
            "n": request.count,
            "size": request.size,
        });
        insert_optional(&mut body, "quality", request.quality.as_deref());
        if let Some(output_format) = &request.output_format {
            body["output_format"] = json!(format_to_openai_value(output_format));
        }
        insert_optional_u8(&mut body, "output_compression", request.output_compression);
        insert_optional(&mut body, "background", request.background.as_deref());

        let response = self
            .client
            .post(format!("{}/images/generations", openai_base_url()))
            .bearer_auth(self.api_key()?)
            .json(&body)
            .send()
            .await?;

        let response = parse_openai_response(response).await?;
        let artifacts = save_openai_data(
            &self.client,
            &response.data,
            &request.output_dir,
            "image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux/openai".to_string(), model: Some(model), artifacts })
    }

    async fn edit(
        &self,
        request: EditRequest,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        progress_cb(ProgressUpdate::new("Calling ZenMux OpenAI Images edit API".to_string()));

        let model = request.model.clone().unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string());
        let mut form = Form::new()
            .text("model", model.clone())
            .text("prompt", request.prompt)
            .text("size", request.size);

        form = form.part("image[]", file_part(&request.input)?);
        if let Some(mask) = &request.mask {
            form = form.part("mask", file_part(mask)?);
        }
        if let Some(quality) = request.quality {
            form = form.text("quality", quality);
        }
        if let Some(output_format) = request.output_format.clone() {
            form = form.text("output_format", format_to_openai_value(&output_format));
        }
        if let Some(output_compression) = request.output_compression {
            form = form.text("output_compression", output_compression.to_string());
        }
        if let Some(background) = request.background {
            form = form.text("background", background);
        }

        let response = self
            .client
            .post(format!("{}/images/edits", openai_base_url()))
            .bearer_auth(self.api_key()?)
            .multipart(form)
            .send()
            .await?;

        let response = parse_openai_response(response).await?;
        let artifacts = save_openai_data(
            &self.client,
            &response.data,
            &request.output_dir,
            "edited-image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux/openai".to_string(), model: Some(model), artifacts })
    }
}

pub struct ZenmuxGoogleProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl ZenmuxGoogleProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key, client: http_client() }
    }

    fn api_key(&self) -> Result<&str> {
        self.api_key.as_deref().filter(|key| !key.is_empty()).ok_or_else(|| {
            anyhow!("ZENMUX_API_KEY is required. Run imagegen-kit login --provider zenmux/google")
        })
    }
}

#[async_trait::async_trait]
impl ImageProvider for ZenmuxGoogleProvider {
    fn name(&self) -> &'static str {
        "ZenMux Google Gemini"
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        let model = request.model.clone().unwrap_or_else(|| DEFAULT_GOOGLE_MODEL.to_string());
        if !is_google_model(&model) {
            return Err(anyhow!(
                "zenmux/google only supports Google/Gemini image models. Use --provider zenmux/openai for OpenAI image models such as gpt-image-2."
            ));
        }

        progress_cb(ProgressUpdate::new("Calling ZenMux Google generateContent API".to_string()));
        self.generate_content(request, model).await
    }

    async fn edit(
        &self,
        _request: EditRequest,
        _progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        Err(anyhow!(
            "zenmux/google does not support image editing in imagegen-kit. Use --provider zenmux/openai for OpenAI image editing."
        ))
    }
}

impl ZenmuxGoogleProvider {
    async fn generate_content(
        &self,
        request: GenerateRequest,
        model: String,
    ) -> Result<ImageResult> {
        let body = json!({
            "contents": [
                {
                    "role": "user",
                    "parts": [{ "text": request.prompt }]
                }
            ],
            "generationConfig": {
                "responseModalities": ["TEXT", "IMAGE"]
            }
        });

        let response = self
            .client
            .post(vertex_url(&model, "generateContent"))
            .header("x-goog-api-key", self.api_key()?)
            .json(&body)
            .send()
            .await?;

        let value = parse_json_response(response).await?;
        let images = extract_generate_content_images(&value)?;
        let artifacts = save_base64_images(
            &images,
            &request.output_dir,
            "image",
            request.output_format.as_deref(),
            request.overwrite,
        )?;

        Ok(ImageResult { provider: "zenmux/google".to_string(), model: Some(model), artifacts })
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .expect("failed to create HTTP client")
}

fn openai_base_url() -> String {
    std::env::var("ZENMUX_OPENAI_BASE_URL").unwrap_or_else(|_| ZENMUX_OPENAI_BASE_URL.to_string())
}

fn vertex_base_url() -> String {
    std::env::var("ZENMUX_VERTEX_BASE_URL").unwrap_or_else(|_| ZENMUX_VERTEX_BASE_URL.to_string())
}

fn vertex_url(model: &str, method: &str) -> String {
    format!("{}/{}:{}", vertex_base_url(), vertex_model_path(model), method)
}

fn vertex_model_path(model: &str) -> String {
    if model.starts_with("publishers/")
        || model.starts_with("projects/")
        || model.starts_with("models/")
    {
        return model.to_string();
    }

    if let Some((publisher, name)) = model.split_once('/') {
        format!("publishers/{}/models/{}", publisher, name)
    } else {
        format!("publishers/google/models/{}", model)
    }
}

fn is_google_model(model: &str) -> bool {
    model.starts_with("google/")
        || model.starts_with("gemini")
        || model.contains("/gemini")
        || model.starts_with("publishers/google/")
}

fn insert_optional(body: &mut Value, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        body[key] = json!(value);
    }
}

fn insert_optional_u8(body: &mut Value, key: &str, value: Option<u8>) {
    if let Some(value) = value {
        body[key] = json!(value);
    }
}

async fn parse_json_response(response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(anyhow!("ZenMux API error {}: {}", status, body));
    }

    serde_json::from_str(&body)
        .map_err(|error| anyhow!("Failed to parse ZenMux response: {} - {}", error, body))
}

async fn parse_openai_response(response: reqwest::Response) -> Result<OpenAiImageResponse> {
    let value = parse_json_response(response).await?;
    serde_json::from_value(value)
        .map_err(|error| anyhow!("Failed to parse OpenAI Images response: {}", error))
}

#[derive(Debug, Deserialize)]
struct OpenAiImageResponse {
    data: Vec<OpenAiImageData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiImageData {
    b64_json: Option<String>,
    url: Option<String>,
    revised_prompt: Option<String>,
}

async fn save_openai_data(
    client: &reqwest::Client,
    data: &[OpenAiImageData],
    output_dir: &Path,
    stem: &str,
    requested_format: Option<&str>,
    overwrite: bool,
) -> Result<Vec<ImageArtifact>> {
    fs::create_dir_all(output_dir)?;
    let mut artifacts = Vec::new();

    for (index, item) in data.iter().enumerate() {
        let image_bytes = if let Some(b64) = &item.b64_json {
            BASE64.decode(b64)?
        } else if let Some(url) = &item.url {
            client.get(url).send().await?.error_for_status()?.bytes().await?.to_vec()
        } else {
            return Err(anyhow!(
                "ZenMux response image {} did not include b64_json or url",
                index + 1
            ));
        };

        let mime_type = requested_format
            .map(format_to_mime)
            .unwrap_or_else(|| detect_image_mime(&image_bytes).unwrap_or("image/png").to_string());
        let path = output_path(output_dir, stem, index + 1, mime_extension(&mime_type), overwrite)?;
        fs::write(&path, image_bytes)?;
        artifacts.push(ImageArtifact {
            path,
            mime_type,
            seed: None,
            revised_prompt: item.revised_prompt.clone(),
        });
    }

    Ok(artifacts)
}

#[derive(Debug)]
struct B64Image {
    b64: String,
    mime_type: String,
    revised_prompt: Option<String>,
}

fn extract_generate_content_images(value: &Value) -> Result<Vec<B64Image>> {
    let candidates = value
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("ZenMux generateContent response did not include candidates"))?;

    let mut images = Vec::new();
    let mut text_parts = Vec::new();
    for candidate in candidates {
        let Some(parts) = candidate.pointer("/content/parts").and_then(Value::as_array) else {
            continue;
        };

        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                text_parts.push(text.to_string());
                continue;
            }

            let inline_data = part.get("inlineData").or_else(|| part.get("inline_data"));
            if let Some(inline_data) = inline_data {
                if let Some(data) = inline_data.get("data").and_then(Value::as_str) {
                    images.push(B64Image {
                        b64: data.to_string(),
                        mime_type: inline_data
                            .get("mimeType")
                            .or_else(|| inline_data.get("mime_type"))
                            .and_then(Value::as_str)
                            .unwrap_or("image/png")
                            .to_string(),
                        revised_prompt: None,
                    });
                }
            }
        }
    }

    if images.is_empty() {
        return Err(anyhow!("ZenMux generateContent response contained no inline image data"));
    }

    let revised_prompt = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };
    for image in &mut images {
        image.revised_prompt = revised_prompt.clone();
    }

    Ok(images)
}

fn save_base64_images(
    images: &[B64Image],
    output_dir: &Path,
    stem: &str,
    requested_format: Option<&str>,
    overwrite: bool,
) -> Result<Vec<ImageArtifact>> {
    fs::create_dir_all(output_dir)?;
    let mut artifacts = Vec::new();

    for (index, image) in images.iter().enumerate() {
        let image_bytes = BASE64.decode(&image.b64)?;
        let mime_type =
            requested_format.map(format_to_mime).unwrap_or_else(|| image.mime_type.clone());
        let path = output_path(output_dir, stem, index + 1, mime_extension(&mime_type), overwrite)?;
        fs::write(&path, image_bytes)?;
        artifacts.push(ImageArtifact {
            path,
            mime_type,
            seed: None,
            revised_prompt: image.revised_prompt.clone(),
        });
    }

    Ok(artifacts)
}

fn output_path(
    output_dir: &Path,
    stem: &str,
    index: usize,
    extension: &str,
    overwrite: bool,
) -> Result<PathBuf> {
    let path = output_dir.join(format!("{}-{}.{}", stem, index, extension));
    if path.exists() && !overwrite {
        return Err(anyhow!(
            "Output file already exists: {}. Pass --overwrite to replace it.",
            path.display()
        ));
    }
    Ok(path)
}

fn file_part(path: &Path) -> Result<Part> {
    let bytes = fs::read(path)?;
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("image").to_string();
    let mime_type = mime_for_path(path);
    Ok(Part::bytes(bytes).file_name(file_name).mime_str(&mime_type)?)
}

fn mime_for_path(path: &Path) -> String {
    mime_guess::from_path(path).first_or_octet_stream().to_string()
}

fn format_to_mime(format: &str) -> String {
    match format.trim().to_lowercase().as_str() {
        "png" | "image/png" => "image/png".to_string(),
        "jpg" | "jpeg" | "image/jpg" | "image/jpeg" => "image/jpeg".to_string(),
        "webp" | "image/webp" => "image/webp".to_string(),
        other if other.starts_with("image/") => other.to_string(),
        other => format!("image/{}", other),
    }
}

fn format_to_openai_value(format: &str) -> String {
    match format.trim().to_lowercase().as_str() {
        "image/png" => "png".to_string(),
        "image/jpg" | "image/jpeg" => "jpeg".to_string(),
        "image/webp" => "webp".to_string(),
        other => other.to_string(),
    }
}

fn mime_extension(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

use super::{
    EditRequest, GenerateRequest, ImageArtifact, ImageProvider, ImageResult, ProgressUpdate,
};
use crate::error::{anyhow, Result};
use crate::models::{resolve_model, GoogleMethod, ModelEntry, ModelOperation};
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
const RESPONSE_PREVIEW_LIMIT: usize = 4000;
const RESPONSE_STRING_LIMIT: usize = 240;

pub struct ZenmuxProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl ZenmuxProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key, client: http_client() }
    }

    fn api_key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| anyhow!("ZENMUX_API_KEY is required. Run imagegen-kit provider --login"))
    }
    async fn generate_openai_images(
        &self,
        request: GenerateRequest,
        model_entry: ModelEntry,
        progress_cb: &mut (dyn FnMut(ProgressUpdate) + Send),
    ) -> Result<ImageResult> {
        let api_model = model_entry.api_model.clone();
        let mut body = json!({
            "model": api_model,
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
        progress_cb(ProgressUpdate::new("Saving generated images".to_string()));
        let artifacts = save_openai_data(
            &self.client,
            &response.data,
            &request.output_dir,
            "image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux".to_string(), model: Some(model_entry.id), artifacts })
    }

    async fn edit_openai_images(
        &self,
        request: EditRequest,
        model_entry: ModelEntry,
        progress_cb: &mut (dyn FnMut(ProgressUpdate) + Send),
    ) -> Result<ImageResult> {
        let api_model = model_entry.api_model.clone();
        let mut form = Form::new()
            .text("model", api_model)
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
        progress_cb(ProgressUpdate::new("Saving edited images".to_string()));
        let artifacts = save_openai_data(
            &self.client,
            &response.data,
            &request.output_dir,
            "edited-image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux".to_string(), model: Some(model_entry.id), artifacts })
    }

    async fn generate_content(
        &self,
        request: GenerateRequest,
        model_entry: ModelEntry,
        progress_cb: &mut (dyn FnMut(ProgressUpdate) + Send),
    ) -> Result<ImageResult> {
        let api_model = model_entry.api_model.clone();
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
            .post(vertex_url(&api_model, "generateContent"))
            .header("x-goog-api-key", self.api_key()?)
            .json(&body)
            .send()
            .await?;

        let value = parse_json_response(response).await?;
        progress_cb(ProgressUpdate::new("Saving generated images".to_string()));
        let images = extract_generate_content_images(&value)?;
        let artifacts = save_base64_images(
            &self.client,
            &images,
            &request.output_dir,
            "image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux".to_string(), model: Some(model_entry.id), artifacts })
    }

    async fn generate_images(
        &self,
        request: GenerateRequest,
        model_entry: ModelEntry,
        progress_cb: &mut (dyn FnMut(ProgressUpdate) + Send),
    ) -> Result<ImageResult> {
        let api_model = model_entry.api_model.clone();
        let body = vertex_generate_images_body(&request);
        let response = self
            .client
            .post(vertex_url(&api_model, "predict"))
            .header("x-goog-api-key", self.api_key()?)
            .json(&body)
            .send()
            .await?;

        let value = parse_json_response(response).await?;
        progress_cb(ProgressUpdate::new("Saving generated images".to_string()));
        let images = extract_vertex_prediction_images(&value)?;
        let artifacts = save_base64_images(
            &self.client,
            &images,
            &request.output_dir,
            "image",
            request.output_format.as_deref(),
            request.overwrite,
        )
        .await?;

        Ok(ImageResult { provider: "zenmux".to_string(), model: Some(model_entry.id), artifacts })
    }
}

#[async_trait::async_trait]
impl ImageProvider for ZenmuxProvider {
    fn name(&self) -> &'static str {
        "ZenMux"
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        let model_entry =
            resolve_model("zenmux", request.model.as_deref(), ModelOperation::Generate)?;

        if is_openai_images_model(&model_entry) {
            progress_cb(ProgressUpdate::new("Waiting for ZenMux OpenAI Images API".to_string()));
            return self.generate_openai_images(request, model_entry, &mut progress_cb).await;
        }

        match model_entry.google_method.ok_or_else(|| {
            anyhow!("Model '{}' does not have a ZenMux endpoint route.", model_entry.id)
        })? {
            GoogleMethod::GenerateContent => {
                progress_cb(ProgressUpdate::new(
                    "Waiting for ZenMux Google generateContent API".to_string(),
                ));
                self.generate_content(request, model_entry, &mut progress_cb).await
            }
            GoogleMethod::Predict => {
                progress_cb(ProgressUpdate::new(
                    "Waiting for ZenMux Google Imagen predict API".to_string(),
                ));
                self.generate_images(request, model_entry, &mut progress_cb).await
            }
        }
    }

    async fn edit(
        &self,
        request: EditRequest,
        mut progress_cb: Box<dyn FnMut(ProgressUpdate) + Send>,
    ) -> Result<ImageResult> {
        let model_entry = resolve_model("zenmux", request.model.as_deref(), ModelOperation::Edit)?;

        if !is_openai_images_model(&model_entry) {
            return Err(anyhow!(
                "Model '{}' supports image editing, but imagegen-kit does not have an edit endpoint route for it.",
                model_entry.id
            ));
        }

        progress_cb(ProgressUpdate::new("Waiting for ZenMux OpenAI Images edit API".to_string()));
        self.edit_openai_images(request, model_entry, &mut progress_cb).await
    }
}

fn is_openai_images_model(model_entry: &ModelEntry) -> bool {
    model_entry.protocols.iter().any(|protocol| protocol == "openai-images")
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

fn vertex_generate_images_body(request: &GenerateRequest) -> Value {
    let mut body = json!({
        "instances": [{ "prompt": request.prompt }],
        "parameters": {
            "sampleCount": request.count
        },
        "imageSize": request.size,
    });

    if let Some(negative_prompt) = &request.negative_prompt {
        body["parameters"]["negativePrompt"] = json!(negative_prompt);
    }
    insert_optional(&mut body, "quality", request.quality.as_deref());
    insert_vertex_output_options(
        &mut body,
        request.output_format.as_deref(),
        request.output_compression,
    );
    body
}

fn insert_vertex_output_options(
    body: &mut Value,
    output_format: Option<&str>,
    output_compression: Option<u8>,
) {
    if output_format.is_none() && output_compression.is_none() {
        return;
    }

    let output_options = &mut body["parameters"]["outputOptions"];
    if let Some(output_format) = output_format {
        output_options["mimeType"] = json!(format_to_mime(output_format));
    }
    if let Some(output_compression) = output_compression {
        output_options["compressionQuality"] = json!(output_compression);
    }
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
    b64: Option<String>,
    url: Option<String>,
    mime_type: String,
    revised_prompt: Option<String>,
}

fn extract_vertex_prediction_images(value: &Value) -> Result<Vec<B64Image>> {
    let predictions = value.get("predictions").and_then(Value::as_array).ok_or_else(|| {
        anyhow!(
            "ZenMux Vertex response did not include predictions.\nResponse preview:\n{}",
            response_preview(value)
        )
    })?;

    let mut images = Vec::new();
    for prediction in predictions {
        if let Some(b64) = prediction.get("bytesBase64Encoded").and_then(Value::as_str) {
            images.push(B64Image {
                b64: Some(b64.to_string()),
                url: None,
                mime_type: prediction
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png")
                    .to_string(),
                revised_prompt: prediction
                    .get("prompt")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        } else if let Some(image) = prediction.get("image") {
            if let Some(b64) = image.get("bytesBase64Encoded").and_then(Value::as_str) {
                images.push(B64Image {
                    b64: Some(b64.to_string()),
                    url: None,
                    mime_type: image
                        .get("mimeType")
                        .and_then(Value::as_str)
                        .unwrap_or("image/png")
                        .to_string(),
                    revised_prompt: prediction
                        .get("prompt")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                });
            }
        } else if let Some(gcs_uri) = prediction.get("gcsUri").and_then(Value::as_str) {
            images.push(B64Image {
                b64: None,
                url: Some(gcs_uri.to_string()),
                mime_type: prediction
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png")
                    .to_string(),
                revised_prompt: prediction
                    .get("prompt")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
    }

    if images.is_empty() {
        return Err(anyhow!(
            "ZenMux Vertex response contained no image bytes.\nResponse preview:\n{}",
            response_preview(value)
        ));
    }

    Ok(images)
}

fn extract_generate_content_images(value: &Value) -> Result<Vec<B64Image>> {
    let candidates = value.get("candidates").and_then(Value::as_array).ok_or_else(|| {
        anyhow!(
            "ZenMux generateContent response did not include candidates.\nResponse preview:\n{}",
            response_preview(value)
        )
    })?;

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
                        b64: Some(data.to_string()),
                        url: None,
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
        return Err(anyhow!(
            "ZenMux generateContent response contained no inline image data.\nResponse preview:\n{}",
            response_preview(value)
        ));
    }

    let revised_prompt = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };
    for image in &mut images {
        image.revised_prompt = revised_prompt.clone();
    }

    Ok(images)
}

fn response_preview(value: &Value) -> String {
    let redacted = redact_response_value(None, value);
    let preview = serde_json::to_string_pretty(&redacted).unwrap_or_else(|_| redacted.to_string());
    truncate_chars(&preview, RESPONSE_PREVIEW_LIMIT)
}

fn redact_response_value(key: Option<&str>, value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| redact_response_value(None, item)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_response_value(Some(key), value)))
                .collect(),
        ),
        Value::String(text) if should_redact_string(key, text) => {
            Value::String(format!("[{} chars redacted]", text.chars().count()))
        }
        _ => value.clone(),
    }
}

fn should_redact_string(key: Option<&str>, text: &str) -> bool {
    if text.chars().count() > RESPONSE_STRING_LIMIT {
        return true;
    }

    let Some(key) = key else {
        return false;
    };
    let key = key.to_ascii_lowercase();
    let image_like_key = key.contains("base64")
        || key.contains("b64")
        || key.contains("bytes")
        || key == "data"
        || key == "image";

    image_like_key && text.len() > 32
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    let truncated = text.chars().take(limit).collect::<String>();
    format!("{truncated}\n... [response preview truncated]")
}

async fn save_base64_images(
    client: &reqwest::Client,
    images: &[B64Image],
    output_dir: &Path,
    stem: &str,
    requested_format: Option<&str>,
    overwrite: bool,
) -> Result<Vec<ImageArtifact>> {
    fs::create_dir_all(output_dir)?;
    let mut artifacts = Vec::new();

    for (index, image) in images.iter().enumerate() {
        let image_bytes: Vec<u8> = if let Some(b64) = &image.b64 {
            BASE64.decode(b64)?
        } else if let Some(url) = &image.url {
            client
                .get(url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|e| anyhow!("Failed to download image from GCS URI: {}", e))?
                .bytes()
                .await
                .map_err(|e| anyhow!("Failed to read image bytes from GCS URI: {}", e))?
                .to_vec()
        } else {
            return Err(anyhow!(
                "ZenMux Vertex prediction {} had neither base64 data nor a URL",
                index + 1
            ));
        };
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

#[cfg(test)]
mod tests {
    use super::{extract_vertex_prediction_images, response_preview};
    use serde_json::json;

    #[test]
    fn includes_vertex_response_preview_when_images_are_missing() {
        let value = json!({
            "predictions": [
                {
                    "error": "model returned no image",
                    "safetyAttributes": {
                        "blocked": true
                    }
                }
            ]
        });

        let error = extract_vertex_prediction_images(&value).unwrap_err().to_string();

        assert!(error.contains("ZenMux Vertex response contained no image bytes"));
        assert!(error.contains("Response preview"));
        assert!(error.contains("model returned no image"));
        assert!(error.contains("blocked"));
    }

    #[test]
    fn extracts_gcs_uri_from_vertex_prediction() {
        let value = json!({
            "predictions": [
                {
                    "gcsUri": "https://example.com/image.png"
                }
            ]
        });

        let images = extract_vertex_prediction_images(&value).unwrap();
        assert_eq!(images.len(), 1);
        assert!(images[0].b64.is_none());
        assert_eq!(images[0].url.as_deref(), Some("https://example.com/image.png"));
    }

    #[test]
    fn redacts_large_response_strings_in_preview() {
        let value = json!({
            "predictions": [
                {
                    "unexpectedImagePayload": "a".repeat(512)
                }
            ],
            "message": "short text"
        });

        let preview = response_preview(&value);

        assert!(preview.contains("[512 chars redacted]"));
        assert!(preview.contains("short text"));
        assert!(!preview.contains(&"a".repeat(512)));
    }
}

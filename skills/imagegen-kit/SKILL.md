---
name: imagegen-kit
description: >-
  Use the imagegen-kit CLI for image generation and image editing workflows
  through ZenMux OpenAI Images and ZenMux Google Gemini / Vertex AI providers.
  Use --dry-run to validate command shape, output paths, provider selection,
  and credential setup without calling external APIs.
---

# imagegen-kit

`imagegen-kit` is a Rust CLI scaffold for image generation workflows.

## Readiness Check

```bash
imagegen-kit --version && imagegen-kit providers && imagegen-kit --help
```

## Providers

- `zenmux/openai`: OpenAI Images protocol, default model `gpt-image-2`
- `zenmux/google`: Google Gemini / Vertex AI protocol, default generate model `google/gemini-3-pro-image-preview`

Both providers use `ZENMUX_API_KEY`.

## Usage

```bash
imagegen-kit generate "prompt text" --provider zenmux/openai --model gpt-image-2
imagegen-kit generate "prompt text" --provider zenmux/google --model google/gemini-3-pro-image-preview
imagegen-kit edit ./input.png "edit prompt" --provider zenmux/openai --model gpt-image-2
imagegen-kit generate "prompt text" --dry-run --json
```

## Credentials

```bash
imagegen-kit login --provider zenmux/openai
imagegen-kit login --list
```

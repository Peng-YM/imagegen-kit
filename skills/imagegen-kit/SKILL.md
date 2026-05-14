---
name: imagegen-kit
description: >-
  Use the imagegen-kit CLI for image generation and image editing workflows
  through ZenMux OpenAI Images, Google Gemini, and Google Imagen endpoints.
  Use --dry-run to validate command shape, output paths, model selection,
  and credential setup without calling external APIs.
---

# imagegen-kit

`imagegen-kit` is a Rust CLI for image generation workflows.

## Readiness Check

```bash
imagegen-kit --version && imagegen-kit provider --list && imagegen-kit --help
```

## Provider

- `zenmux`: one ZenMux login, default generate/edit model `gpt-image-2`

ZenMux uses `ZENMUX_API_KEY`.
OpenAI image models must route through the OpenAI Images endpoint; the CLI chooses that endpoint from model metadata.
Model metadata comes from the embedded `models.json` catalog.

## Usage

```bash
imagegen-kit generate "prompt text" --model gpt-image-2
imagegen-kit generate "prompt text" --model gpt-image-2 --show
imagegen-kit generate "prompt text" --model google/gemini-3-pro-image-preview
imagegen-kit generate "prompt text" --model qwen/qwen-image-2.0
imagegen-kit edit ./input.png "edit prompt" --model gpt-image-2 --show
imagegen-kit generate "prompt text" --dry-run --json
imagegen-kit provider --list --provider zenmux
imagegen-kit status
```

## Credentials

```bash
imagegen-kit provider --login
imagegen-kit provider --login --provider zenmux
imagegen-kit provider --logout --provider zenmux
imagegen-kit status --json
```

# imagegen-kit

<p align="center">
  <a href="https://github.com/Peng-YM/imagegen-kit/stargazers">
    <img src="https://img.shields.io/github/stars/Peng-YM/imagegen-kit?style=flat-square" alt="Stars">
  </a>
  <a href="https://github.com/Peng-YM/imagegen-kit/network/members">
    <img src="https://img.shields.io/github/forks/Peng-YM/imagegen-kit?style=flat-square" alt="Forks">
  </a>
  <a href="https://github.com/Peng-YM/imagegen-kit/issues">
    <img src="https://img.shields.io/github/issues/Peng-YM/imagegen-kit?style=flat-square" alt="Issues">
  </a>
  <a href="https://github.com/Peng-YM/imagegen-kit/blob/master/LICENSE">
    <img src="https://img.shields.io/github/license/Peng-YM/imagegen-kit?style=flat-square" alt="License">
  </a>
  <a href="https://github.com/Peng-YM/imagegen-kit/releases">
    <img src="https://img.shields.io/github/v/release/Peng-YM/imagegen-kit?style=flat-square" alt="Release">
  </a>
  <a href="https://github.com/Peng-YM/imagegen-kit/releases">
    <img src="https://img.shields.io/github/downloads/Peng-YM/imagegen-kit/total?style=flat-square" alt="Downloads">
  </a>
</p>

`imagegen-kit` is a Rust CLI for image generation workflows, with provider boundaries, encrypted credential storage, JSON output, and dry-run support.

The provider integration targets ZenMux:

- `zenmux`: one ZenMux login, with model-based routing to OpenAI Images, Google Gemini, or Google Imagen endpoints

ZenMux uses `ZENMUX_API_KEY`.
OpenAI image models are routed only through the OpenAI Images endpoint, even if ZenMux model metadata also lists Google Imagen compatibility.
Model metadata is stored in [`models.json`](./models.json) and embedded into the binary at compile time.

## Current Scope

- CLI commands for generation, editing, credential management, and provider listing
- ZenMux provider with model-based endpoint routing
- Encrypted local credential storage
- Dry-run output for validating command shape before real API calls
- JSON output for agent and script usage

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/imagegen-kit/master/install.sh | bash
```

To install a specific release:

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/imagegen-kit/master/install.sh | bash -s -- v0.3.0
```

## Usage

```bash
# Store a ZenMux API key
imagegen-kit provider --login

# Show logged-in providers, their models, and generate/edit defaults
imagegen-kit status

# Generate via OpenAI Images endpoint
imagegen-kit generate "a clean product photo of a ceramic mug" \
  --model gpt-image-2 \
  --quality high

# Generate and show the saved image inline in iTerm2 or Kitty
imagegen-kit generate "a clean product photo of a ceramic mug" --show

# Generate via Google Gemini endpoint
imagegen-kit generate "a nano banana dish in a fancy restaurant" \
  --model google/gemini-3-pro-image-preview

# Generate via Google Imagen endpoint
imagegen-kit generate "a clean product render" \
  --model qwen/qwen-image-2.0

# Preview without calling ZenMux
imagegen-kit generate "a clean product photo of a ceramic mug" --dry-run --json
```

## Commands

### `generate`

Preview or execute a text-to-image request.
When `--output-dir` is omitted, images are saved to a new random directory under the system temp directory.

```bash
imagegen-kit generate "prompt text" \
  --model gpt-image-2 \
  --size 1024x1024 \
  --quality high \
  --output-format png \
  --count 1 \
  --output-dir ./output \
  --show
```

### `edit`

Preview or execute an image editing request.
When `--output-dir` is omitted, images are saved to a new random directory under the system temp directory.

```bash
imagegen-kit edit ./input.png "edit prompt" \
  --mask ./mask.png \
  --model gpt-image-2 \
  --size 1024x1024 \
  --output-dir ./output \
  --show
```

`--show` displays saved images inline in iTerm2 or Kitty only. It is not available with `--json` or `--quiet`.

### `provider`

List provider model metadata and manage encrypted provider credentials.

```bash
imagegen-kit provider --list
imagegen-kit provider --list --provider zenmux
imagegen-kit provider --login
imagegen-kit provider --login --provider zenmux --api-key "$ZENMUX_API_KEY"
imagegen-kit provider --logout --provider zenmux
```

### `status`

List currently logged-in providers, their available models, and default models for generate/edit modes.

```bash
imagegen-kit status
imagegen-kit status --json
```

## Provider Notes

### `zenmux`

Uses one ZenMux credential and routes each model according to embedded model metadata.

- OpenAI Images base URL: `https://zenmux.ai/api/v1`
- OpenAI generate endpoint: `/images/generations`
- OpenAI edit endpoint: `/images/edits`
- Google Gemini / Imagen base URL: `https://zenmux.ai/api/vertex-ai/v1`
- Supports Gemini image models through `:generateContent`
- Supports non-OpenAI Imagen catalog models through `:predict`
- Does not route OpenAI image models through the Google protocol
- Default generate model: `gpt-image-2`
- Default edit model: `gpt-image-2`
- OpenAI endpoint auth: `Authorization: Bearer $ZENMUX_API_KEY`
- Google endpoint auth: `x-goog-api-key: $ZENMUX_API_KEY`

Model routing comes from [`models.json`](./models.json). The file remains readable in the repository, and the Rust code embeds it with `include_str!("../models.json")`, so released binaries do not need a separate runtime copy.

Firecrawl snapshots of the ZenMux docs used for this implementation are saved in:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`
- `.firecrawl/zenmux-models-imagen.md`
- `.firecrawl/zenmux-models-imagen-images.md`

## Contributing

For source builds, development checks, and project layout notes, see [CONTRIBUTING.md](./CONTRIBUTING.md).

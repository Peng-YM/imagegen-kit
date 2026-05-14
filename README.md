# imagegen-kit

`imagegen-kit` is a Rust CLI for image generation workflows, with provider boundaries, encrypted credential storage, JSON output, and dry-run support.

The first provider integrations target ZenMux:

- `zenmux/openai`: ZenMux OpenAI Images protocol, `https://zenmux.ai/api/v1/images/*`
- `zenmux/google`: ZenMux Google Gemini / Vertex AI protocol, `https://zenmux.ai/api/vertex-ai/v1/*`

Both providers use `ZENMUX_API_KEY`.
OpenAI image models must use `zenmux/openai`; `zenmux/google` supports Gemini and non-OpenAI Imagen models only.
Model metadata is stored in [`models.json`](./models.json) and embedded into the binary at compile time.

## Current Scope

- CLI commands for generation, editing, credential management, and provider listing
- ZenMux OpenAI Images provider
- ZenMux Google Gemini / Imagen / Vertex AI provider
- Encrypted local credential storage
- Dry-run output for validating command shape before real API calls
- JSON output for agent and script usage

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/imagegen-kit/master/install.sh | bash
```

To install a specific release:

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/imagegen-kit/master/install.sh | bash -s -- v0.1.0
```

## Usage

```bash
# Store a ZenMux API key
imagegen-kit provider --login

# Show logged-in providers, their models, and generate/edit defaults
imagegen-kit status

# Generate via OpenAI Images protocol
imagegen-kit generate "a clean product photo of a ceramic mug" \
  --provider zenmux/openai \
  --model gpt-image-2 \
  --quality high

# Generate and show the saved image inline in iTerm2 or Kitty
imagegen-kit generate "a clean product photo of a ceramic mug" --show

# Generate via Google Gemini protocol
imagegen-kit generate "a nano banana dish in a fancy restaurant" \
  --provider zenmux/google \
  --model google/gemini-3-pro-image-preview

# Generate via Google Imagen protocol
imagegen-kit generate "a clean product render" \
  --provider zenmux/google \
  --model qwen/qwen-image-2.0

# Preview without calling ZenMux
imagegen-kit generate "a clean product photo of a ceramic mug" --dry-run --json
```

## Commands

### `generate`

Preview or execute a text-to-image request.

```bash
imagegen-kit generate "prompt text" \
  --provider zenmux/openai \
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

```bash
imagegen-kit edit ./input.png "edit prompt" \
  --mask ./mask.png \
  --provider zenmux/openai \
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
imagegen-kit provider --list --provider zenmux/google
imagegen-kit provider --login
imagegen-kit provider --login --provider zenmux/openai
imagegen-kit provider --login --provider zenmux/google --api-key "$ZENMUX_API_KEY"
imagegen-kit provider --logout --provider zenmux/openai
```

### `status`

List currently logged-in providers, their available models, and default models for generate/edit modes.

```bash
imagegen-kit status
imagegen-kit status --json
```

## Provider Notes

### `zenmux/openai`

Uses the OpenAI Images protocol documented by ZenMux.

- Base URL: `https://zenmux.ai/api/v1`
- Generate endpoint: `/images/generations`
- Edit endpoint: `/images/edits`
- Default generate model: `gpt-image-2`
- Default edit model: `gpt-image-2`
- Auth: `Authorization: Bearer $ZENMUX_API_KEY`

### `zenmux/google`

Uses the Google Gemini / Imagen / Vertex AI protocol documented by ZenMux.

- Base URL: `https://zenmux.ai/api/vertex-ai/v1`
- Supports Google/Gemini image models through `:generateContent`
- Supports non-OpenAI Imagen catalog models through `:predict`
- Does not route OpenAI image models through the Google protocol; use `zenmux/openai` for `gpt-image-*`
- Default generate model: `google/gemini-3-pro-image-preview`
- Default edit model: none
- Image editing is not exposed through `zenmux/google` in this CLI
- Auth: `x-goog-api-key: $ZENMUX_API_KEY`

Model routing comes from [`models.json`](./models.json). The file remains readable in the repository, and the Rust code embeds it with `include_str!("../models.json")`, so released binaries do not need a separate runtime copy.

Firecrawl snapshots of the ZenMux docs used for this implementation are saved in:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`
- `.firecrawl/zenmux-models-imagen.md`
- `.firecrawl/zenmux-models-imagen-images.md`

## Contributing

For source builds, development checks, and project layout notes, see [CONTRIBUTING.md](./CONTRIBUTING.md).

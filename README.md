# imagegen-kit

`imagegen-kit` is a Rust CLI for image generation workflows. It mirrors the structure of `pdf-to-markdown`: a small command-line entrypoint, reusable library modules, provider boundaries, encrypted credential storage, cache management, JSON output, and dry-run support.

The first provider integrations target ZenMux:

- `zenmux/openai`: ZenMux OpenAI Images protocol, `https://zenmux.ai/api/v1/images/*`
- `zenmux/google`: ZenMux Google Gemini / Vertex AI protocol, `https://zenmux.ai/api/vertex-ai/v1/*`

Both providers use `ZENMUX_API_KEY`.

## Current Scope

- CLI commands for generation, editing, credential management, provider listing, and cache management
- ZenMux OpenAI Images provider
- ZenMux Google Gemini / Vertex AI provider
- Encrypted local credential storage
- Cache index scaffolding
- Dry-run output for validating command shape before real API calls
- JSON output for agent and script usage
- Release/build skeleton copied in spirit from `pdf-to-markdown`

## Install From Source

```bash
cargo install --path .
```

## Usage

```bash
# Store a ZenMux API key
imagegen-kit login --provider zenmux/openai

# Generate via OpenAI Images protocol
imagegen-kit generate "a clean product photo of a ceramic mug" \
  --provider zenmux/openai \
  --model gpt-image-2 \
  --quality high

# Generate via Google Gemini protocol
imagegen-kit generate "a nano banana dish in a fancy restaurant" \
  --provider zenmux/google \
  --model google/gemini-3-pro-image-preview

# Preview without calling ZenMux
imagegen-kit generate "a clean product photo of a ceramic mug" --dry-run --json

# Inspect cache state
imagegen-kit cache status
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
  --output-dir ./output
```

### `edit`

Preview or execute an image editing request.

```bash
imagegen-kit edit ./input.png "edit prompt" \
  --mask ./mask.png \
  --provider zenmux/openai \
  --model gpt-image-2 \
  --size 1024x1024 \
  --output-dir ./output
```

### `login`

Manage encrypted provider credentials.

```bash
imagegen-kit login --provider zenmux/openai
imagegen-kit login --provider zenmux/google --api-key "$ZENMUX_API_KEY"
imagegen-kit login --list
imagegen-kit login --delete zenmux
```

### `cache`

Inspect or clear the local cache index.

```bash
imagegen-kit cache status
imagegen-kit cache clear --force
```

## Provider Notes

### `zenmux/openai`

Uses the OpenAI Images protocol documented by ZenMux.

- Base URL: `https://zenmux.ai/api/v1`
- Generate endpoint: `/images/generations`
- Edit endpoint: `/images/edits`
- Default model: `gpt-image-2`
- Auth: `Authorization: Bearer $ZENMUX_API_KEY`

### `zenmux/google`

Uses the Google Gemini / Vertex AI protocol documented by ZenMux.

- Base URL: `https://zenmux.ai/api/vertex-ai/v1`
- Google image models use `:generateContent`
- Non-Google image models such as `openai/gpt-image-2` use `:predict`
- Default generate model: `google/gemini-3-pro-image-preview`
- Default edit model: `openai/gpt-image-2`
- Auth: `x-goog-api-key: $ZENMUX_API_KEY`

Firecrawl snapshots of the ZenMux docs used for this implementation are saved in:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`

## Development

```bash
make fmt
make check
make clippy
make test
```

# Contributing

## Build From Source

```bash
cargo install --path .
```

## Development Checks

```bash
make fmt
make check
make clippy
make test
```

## Project Layout

- `src/main.rs` owns CLI parsing, output modes, and process exit behavior.
- `src/lib.rs` exports the reusable library surface.
- `src/provider/` owns provider traits and provider-specific implementations.
- `src/auth.rs` owns encrypted credential storage.
- `src/utils.rs` owns small shared helpers.

Keep provider SDK details out of `main.rs`; wire them through `ImageProvider`.

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

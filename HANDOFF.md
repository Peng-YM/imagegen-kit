# Handoff

`imagegen-kit` is currently a compile-ready Rust CLI with a ZenMux image provider.

Implemented provider ID:

1. `zenmux`: one ZenMux login, default generate/edit model `gpt-image-2`.

OpenAI image models are intentionally not routed through the Google protocol; model metadata routes them through the OpenAI Images endpoint. Gemini and non-OpenAI Imagen models route through the ZenMux Google endpoint.
Model metadata lives in `models.json` and is embedded into the binary via `include_str!("../models.json")`; released binaries do not need a separate runtime copy.
Provider metadata and credential login/logout are handled by `imagegen-kit provider`; use `provider --list` for model descriptions and `provider --login` / `provider --logout` for credential storage.
Logged-in providers and generate/edit defaults are reported by `imagegen-kit status`.
Generation and editing support `--show` for inline image display in iTerm2 or Kitty.

Firecrawl source docs:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`
- `.firecrawl/zenmux-models-imagen.md`
- `.firecrawl/zenmux-models-imagen-images.md`

Open questions:

1. Whether to add multiple reference image inputs for `edit`.
2. Whether to add streaming image generation.
3. Whether to publish the bundled agent skill.

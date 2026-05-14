# Handoff

`imagegen-kit` is currently a compile-ready Rust CLI with ZenMux image providers.

Implemented provider IDs:

1. `zenmux/openai`: OpenAI Images protocol, default model `gpt-image-2`.
2. `zenmux/google`: Google Gemini / Imagen / Vertex AI protocol, default generate model `google/gemini-3-pro-image-preview`.

OpenAI image models are intentionally not routed through the Google protocol. Use `zenmux/openai` for `gpt-image-*`.
Model metadata lives in `models.json` and is embedded into the binary via `include_str!("../models.json")`; released binaries do not need a separate runtime copy.
Provider metadata and credential login are handled by `imagegen-kit provider`; use `provider --list` for model descriptions and `provider --login` for interactive credential storage.
Logged-in providers and generate/edit defaults are reported by `imagegen-kit status`.

Firecrawl source docs:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`
- `.firecrawl/zenmux-models-imagen.md`
- `.firecrawl/zenmux-models-imagen-images.md`

Open questions:

1. Whether to add multiple reference image inputs for `edit`.
2. Whether to add streaming image generation.
3. Whether to publish the bundled agent skill.

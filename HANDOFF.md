# Handoff

`imagegen-kit` is currently a compile-ready Rust CLI with ZenMux image providers.

Implemented provider IDs:

1. `zenmux/openai`: OpenAI Images protocol, default model `gpt-image-2`.
2. `zenmux/google`: Google Gemini / Vertex AI protocol, default generate model `google/gemini-3-pro-image-preview`.

OpenAI image models are intentionally not routed through the Google protocol. Use `zenmux/openai` for `gpt-image-*`.

Firecrawl source docs:

- `.firecrawl/zenmux-image-generation.md`
- `.firecrawl/zenmux-openai-image-generation.md`

Open questions:

1. Whether to add multiple reference image inputs for `edit`.
2. Whether to add streaming image generation.
3. Whether to publish the bundled agent skill.

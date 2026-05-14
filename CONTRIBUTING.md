# Contributing

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
- `src/cache.rs` owns cache indexing and cache management.
- `src/utils.rs` owns small shared helpers.

Keep provider SDK details out of `main.rs`; wire them through `ImageProvider`.

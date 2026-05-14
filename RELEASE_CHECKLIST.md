# Release Checklist

1. Update `version` in `Cargo.toml`.
2. Run `make fmt-check`.
3. Run `make clippy`.
4. Run `make test`.
5. Run `make release`.
6. Run `./github_release.sh`.

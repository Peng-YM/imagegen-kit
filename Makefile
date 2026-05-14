.PHONY: all build install release clean fmt fmt-check clippy check test size

all: build

build:
	@cargo build

install:
	@cargo install --path .

release:
	@cargo build --release

clean:
	@cargo clean

fmt:
	@cargo fmt

fmt-check:
	@cargo fmt --check

clippy:
	@cargo clippy -- -D warnings

check:
	@cargo check

test:
	@cargo test

size:
	@cargo build --release
	@ls -lh target/release/imagegen-kit* 2>/dev/null || echo "Binary not found"

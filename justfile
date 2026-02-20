msrv := `cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].rust_version'`

# Run all CI checks
ci: fmt lint test doc

# Check formatting
fmt:
    cargo +{{msrv}} fmt --check

# Lint with clippy
lint:
    cargo +{{msrv}} clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo +{{msrv}} test

# Build docs
doc:
    RUSTDOCFLAGS="-D warnings" cargo +{{msrv}} doc --no-deps

default:
    @just --list

ppy:
    cargo clippy --all-targets --all-features -- -D warnings

fmt:
    cargo fmt --all -- --check

test:
    cargo test --all-features

check: fmt ppy test
    @echo "All checks passed"
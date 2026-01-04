.PHONY: fmt, lint

fmt:
	cargo fmt --all

lint:
	cargo clippy --all -- -D warnings

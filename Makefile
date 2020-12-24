all: test

build:
	@cargo build --all --all-features

doc:
	@cargo doc --all --all-features

test: cargotest

cargotest:
	@rustup component add rustfmt 2> /dev/null
	@cargo test --all

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy

.PHONY: all doc test cargotest format format-check lint

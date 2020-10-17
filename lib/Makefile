
build:
	@cargo build

test:
	@cargo test -- --nocapture

coverage:
	@cargo install cargo-tarpaulin
	@cargo tarpaulin -o Html -v --features testable_internals --exclude-files hbase.rs --exclude-files '*.h' --exclude-files common --  --test-threads=1

docs: build
	@cargo doc --no-deps --document-private-items

style-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy --all-features --all --tests --examples -- -D clippy::all -D warnings


.PHONY: build test docs style-check lint


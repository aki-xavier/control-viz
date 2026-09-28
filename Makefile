.PHONY: test lint serve

test:
	mbx test --workspace

lint:
	mbx fmt --all --check
	mbx clippy --workspace --all-targets -- -D warnings

serve:
	mbx run --release -- $(ARGS)

# control-viz — the scrub player and the static server that feeds it.
#
#   make test    # the route table's answers, byte for byte, over a real socket, then the comment rules
#   make lint    # rustfmt --check + clippy over every target
#   make comments# the comment rules alone, with the local approximation for the rest
#   make serve   # the player on 127.0.0.1:8321, read-only
#
# Requires: mbx (the Cargo build-cache wrapper): every Cargo command below is run as
# `mbx <subcommand>`.
#
# The player page is this crate's viz/ and needs no argument. The other two roots are siblings':
# the meshes default to ../control-model/models — the directory a recording's `/models/<rest>`
# names under — and the recordings to /tmp/simrec, which is where ../g1-biped's g1_stand_viz
# writes them. ARGS passes control-viz's own flags through:
#
#   make serve ARGS="--simrec /tmp/elsewhere --addr 127.0.0.1:8322"

COMMENT_WHY ?= ../comment-why

.PHONY: test lint comments serve

test:
	mbx test --workspace
	$(MAKE) comments

lint:
	mbx fmt --all --check
	mbx clippy --workspace --all-targets -- -D warnings

# The rules read text, so they need nothing this crate builds; the gate inside `mbx test` is
# tests/comment_why.rs and this is the same rules over the working tree.
comments:
	mbx run --quiet --manifest-path $(COMMENT_WHY)/Cargo.toml --bin comment-why -- --review

serve:
	mbx run --release -- $(ARGS)

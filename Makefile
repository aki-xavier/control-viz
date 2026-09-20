# control-viz — the scrub player and the static server that feeds it.
#
#   make test    # the route table's answers, byte for byte, over a real socket
#   make lint    # rustfmt --check + clippy over every target
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

.PHONY: test lint serve

test:
	mbx test --workspace

lint:
	mbx fmt --all --check
	mbx clippy --workspace --all-targets -- -D warnings

serve:
	mbx run --release -- $(ARGS)

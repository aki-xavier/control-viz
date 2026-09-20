# control-viz — the scrub player and the static server that feeds it

A project of its own: the player page, the server that hands it out, and the route table and the
response bytes in between. MIT-licensed (see `LICENSE`). No engine, no model and no dependencies —
`std`'s `TcpListener`, `std::fs`, and nothing else.

It was `z1-arm`'s `examples/viz_serve.rs` and `g1-biped`'s `papers/viz/` until 2026-09-20: the
server had left with the arm and the page with the walk, so neither tree could serve a frame, and
the server resolved both of its roots against its own crate directory (measured then: `/`,
`/index.html`, `/lib/three.min.js` and `/?rec=duck` were all 404). The page and the server are one
thing, and neither machine owns it.

## What is here

```text
src/lib.rs      the route table (resolve), the mime type per extension, the response bytes
                (Response::to_bytes) and the answer to one request (handle)
src/main.rs     the command line and the accept loop: one request per connection — read the
                request line, answer it, close
viz/            the player: index.html (the scrub player — timeline drag, play, speed, orbit
                camera), poke.html (mouse force injection) and lib/, the vendored three.js r160
                whose three files viz/lib/README.md pins by sha256
tests/serve.rs  the oracle: a spawned server, a real socket, and the exact bytes of each answer
```

## The three roots it serves

The server answers three things, and only the first is this crate's:

- **the player** — `/`, `/index.html`, `/poke.html` and `/lib/*`, from this crate's own `viz/`;
- **the recordings** — `/simrec/*.jsonl`, written by the machine side: `../g1-biped`'s `g1_stand_viz`
  writes `/tmp/simrec/<name>.jsonl`, and the player fetches `/simrec/<rec>.jsonl` for its `?rec=`
  parameter (default `duck`);
- **the meshes** — `/models/*`, which are `../control-model`'s data. A recording names them **by
  URL** (`/models/unitree_g1/meshes/pelvis.STL`), so `--models` is the directory those names are
  relative to and not the `models/` parent.

All three are arguments, not paths spelled in the binary: a relative path compiled into a binary is
a claim about a checkout layout that holds only on the machine it was written on.

```text
--viz <dir>         the player page directory                   [<crate>/viz]
--models <dir>      the mesh directory `/models/` names under    [../control-model/models]
--simrec <dir>      the recordings directory                     [/tmp/simrec]
--addr <addr>       the address to serve on                      [127.0.0.1:8321]
--max-requests <n>  stop after n requests (0 = until killed)     [0]
```

A root that is not there is not fatal — the player is only reachable once a recording exists, and
the recording is another project's — so startup warns once per missing root on stderr rather than
refusing to listen.

## Running it

The player needs a recording to scrub, and the recording is the machine's:

```sh
# in ../g1-biped: steps the standing loop on the engine, writes /tmp/simrec/g1_stand.jsonl
mbx run --release --example g1_stand_viz

# here: the player, then open http://127.0.0.1:8321/?rec=g1_stand
make serve
```

`--max-requests` is the stop condition a test wants: the server serves that many requests and exits,
which is what `tests/serve.rs` drives. Its `--addr 127.0.0.1:0` is the other half of it — the OS
chooses a port and the startup line reports the one it bound, so no test picks a port and hopes.

## The route table

| route | path | served from |
|---|---|---|
| `/`, `/index.html`, `/poke.html` | that file | `--viz` |
| `/lib/<rest>` | `<viz>/lib/<rest>` | `--viz` |
| `/simrec/<rest>` | `<simrec>/<rest>` | `--simrec` |
| `/models/<rest>` | `<models>/<rest>` | `--models` |
| anything else | — | 404 |

A URL carrying `..` is refused before any route is applied. The two 404s are distinct on purpose:
`not found: <url>` is a route the table rejects, `missing: <path>` is a file that is not there — and
the second is the one a reader of `?rec=` hits when the recording has not been written. Responses
carry `Connection: close` and one request is served per connection; `tests/serve.rs` compares those
bytes literally, headers included, so a header added or reordered fails rather than surprises.

One departure from the server this replaces: extensions are matched case-insensitively. The model
tree is not consistent about it (the meshes a recording names are `.STL` while their neighbours are
`.stl`), and the old table matched as written, so every recorded mesh went out as `text/plain`.
`STLLoader` parses the bytes and ignores the header, which is why that went unnoticed rather than
why it was right.

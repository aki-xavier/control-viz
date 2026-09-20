//! control-viz — the scrub player and the static server that feeds it.
//!
//! The player is this crate's `viz/`: `index.html` (the scrub player: timeline drag, play, speed,
//! orbit camera) and `poke.html` (the force-injection page), with the vendored three.js under
//! `viz/lib/`. The server hands out three things over a fixed route table:
//!
//! - the player itself (`/`, `/index.html`, `/poke.html`, `/lib/*`),
//! - the recordings the machine side writes (`/simrec/*.jsonl` — `g1-biped`'s `g1_stand_viz`),
//! - the meshes `control-model` owns (`/models/*`).
//!
//! The recordings name the meshes BY URL (`/models/unitree_g1/meshes/pelvis.STL`), which is why the
//! model root is the directory those names are relative to and not the `models/` parent: the tree
//! that holds them is a sibling's, and a path spelled here would be a second copy of that
//! convention. All three directories are the caller's for the same reason ([`Roots`]) — this crate
//! is a leaf and holds no data of theirs.
//!
//! What is pinned here is the answer: the route table, the mime type per extension, and the exact
//! bytes of a response, headers included. `tests/serve.rs` drives a spawned server over a real
//! socket and compares those bytes, one case per shape of answer.

use std::fs;
use std::path::{Path, PathBuf};

/// The address the server listens on unless `--addr` moves it.
pub const DEFAULT_ADDR: &str = "127.0.0.1:8321";

/// The three directories the route table resolves against. None of them is this crate's to know:
/// the player is ours, the meshes are `control-model`'s and the recordings are `g1-biped`'s.
pub struct Roots {
    /// `/`, `/index.html` and `/poke.html` come from here, and `/lib/*` from its `lib/`.
    pub viz: PathBuf,
    /// `/models/<rest>` resolves under here, so a recording's `/models/unitree_g1/x.STL` is
    /// `<models>/unitree_g1/x.STL`.
    pub models: PathBuf,
    /// `/simrec/<rest>` resolves under here.
    pub simrec: PathBuf,
}

/// Why a URL did not resolve to a path.
#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    /// The table has no route for this URL.
    Route,
    /// The URL carries a `..`. Refused before any route is applied, so no route can be talked into
    /// leaving its root.
    Traversal,
}

/// Maps a request URL to a file path: the routes below, checked after the traversal check.
///
/// The query string is dropped first, then the leading component is stripped BEFORE the join:
/// `Path::join` with a second component that starts at the root REPLACES the base instead of
/// extending it, so the routes that used to be spelled by concatenation strip their prefix and join
/// the remainder. `Reject::Traversal` is what keeps a stripped remainder from climbing out.
pub fn resolve(roots: &Roots, url: &str) -> Result<PathBuf, Reject> {
    let p = match url.find('?') {
        Some(i) => &url[..i],
        None => url,
    };
    if p.contains("..") {
        return Err(Reject::Traversal);
    }
    if p == "/" || p == "/index.html" {
        return Ok(roots.viz.join("index.html"));
    }
    if p == "/poke.html" {
        return Ok(roots.viz.join("poke.html"));
    }
    if let Some(rest) = p.strip_prefix("/lib/") {
        return Ok(roots.viz.join("lib").join(rest));
    }
    if let Some(rest) = p.strip_prefix("/simrec/") {
        return Ok(roots.simrec.join(rest));
    }
    if let Some(rest) = p.strip_prefix("/models/") {
        return Ok(roots.models.join(rest));
    }
    Err(Reject::Route)
}

/// The content type of a file by its extension; the fall-through is `text/plain`.
///
/// The comparison is case-insensitive because the model tree is not consistent about it — the meshes
/// a recording names are `.STL` while its neighbours are `.stl`, and the server this one replaces
/// matched the extension as written, so every recorded mesh went out as `text/plain`. `STLLoader`
/// parses the bytes and ignores the header, which is why that went unnoticed rather than why it was
/// right.
pub fn mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "jsonl" | "json" => "application/json",
        "stl" => "application/octet-stream",
        "png" => "image/png",
        "css" => "text/css",
        _ => "text/plain",
    }
}

/// The response's status, optional content type and body.
pub struct Response {
    pub status: u16,
    pub ctype: Option<&'static str>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn reason(&self) -> &'static str {
        match self.status {
            200 => "OK",
            400 => "Bad Request",
            _ => "Not Found",
        }
    }

    /// The exact wire bytes, headers included. `Connection: close` and one request per connection
    /// are the whole of the HTTP this server speaks; `tests/serve.rs` compares these bytes
    /// literally, so a header added or reordered is a test failure rather than a surprise.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, self.reason());
        if let Some(t) = self.ctype {
            out += &format!("Content-Type: {t}\r\n");
        }
        out += &format!("Content-Length: {}\r\n", self.body.len());
        out += "Connection: close\r\n\r\n";
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// Answers one request: the resolved file with its mime type, or one of the two 404 bodies —
/// `not found: <url>` for a route the table rejects, `missing: <path>` for a file not there. The
/// two are distinct on purpose: "the player has no such route" and "the recording has not been
/// written yet" are different mistakes, and the second is the one a reader of `?rec=` hits.
pub fn handle(roots: &Roots, url: &str) -> Response {
    let path = match resolve(roots, url) {
        Ok(path) => path,
        Err(_) => return not_found(format!("not found: {url}")),
    };
    let body = match fs::read(&path) {
        Ok(body) => body,
        Err(_) => return not_found(format!("missing: {}", path.display())),
    };
    Response {
        status: 200,
        ctype: Some(mime(&path)),
        body,
    }
}

/// A 404 with no content type: the two bodies above are text, but they are the server's words
/// rather than a file's, and the player distinguishes them by their text.
fn not_found(body: String) -> Response {
    Response {
        status: 404,
        ctype: None,
        body: body.into_bytes(),
    }
}

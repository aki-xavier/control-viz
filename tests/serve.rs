// serve.rs — the route table's answers, byte for byte.
//
// The oracle here is a spawned server driven over a real socket, not a call into the route table:
// what a reader of `?rec=` gets is the bytes on the wire, headers included, and those are what this
// file pins. One case per shape of answer — the page, the libs, the meshes, the recording, a
// recording that has not been written, a route the table rejects, and a `..` path.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// The fixture's files, written under the integration tests' own temporary directory.
const PAGE: &[u8] = b"<!DOCTYPE html>\n<html><body>player</body></html>\n";
const POKE: &[u8] = b"<!DOCTYPE html>\n<html><body>poke</body></html>\n";
const LIB: &[u8] = b"// three.module.js (the vendored build, truncated for the fixture)\n";
const MESH: &[u8] = b"solid pelvis\nendsolid pelvis\n";
const RECORDING: &[u8] = b"{\"dt\":0.02,\"links\":[],\"statics\":[],\"frames\":[]}\n";

/// Writes the three roots and returns them: the player page, the meshes the recordings name, and
/// the recordings. `name` separates the tests' fixtures — they run in parallel and each owns its
/// own tree.
fn fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("serve_fixture_{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let (viz, models, simrec) = (root.join("viz"), root.join("models"), root.join("simrec"));
    for dir in [
        viz.join("lib"),
        models.join("unitree_g1").join("meshes"),
        simrec.clone(),
    ] {
        std::fs::create_dir_all(&dir).expect("the fixture's directories");
    }
    for (path, bytes) in [
        (viz.join("index.html"), PAGE),
        (viz.join("poke.html"), POKE),
        (viz.join("lib").join("three.module.js"), LIB),
        (
            models.join("unitree_g1").join("meshes").join("pelvis.STL"),
            MESH,
        ),
        (simrec.join("g1_stand.jsonl"), RECORDING),
    ] {
        std::fs::write(&path, bytes).expect("a fixture file");
    }
    (viz, models, simrec)
}

/// The exact bytes of one answer: status line, content type when there is one, length, the
/// connection header, and the body.
fn expected(status: &str, ctype: Option<&str>, body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\n");
    if let Some(t) = ctype {
        out += &format!("Content-Type: {t}\r\n");
    }
    out += &format!("Content-Length: {}\r\n", body.len());
    out += "Connection: close\r\n\r\n";
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// One request on its own connection; the server answers one per connection and closes.
fn request(addr: &str, target: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr).expect("the server's address");
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
        .expect("the request line");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("the response");
    response
}

/// Spawns the server on an OS-chosen port and returns it with the address it reports.
fn spawn(viz: &Path, models: &Path, simrec: &Path, max_requests: usize) -> (Child, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_control-viz"))
        .args([
            "--viz",
            viz.to_str().expect("a utf-8 fixture path"),
            "--models",
            models.to_str().expect("a utf-8 fixture path"),
            "--simrec",
            simrec.to_str().expect("a utf-8 fixture path"),
            "--addr",
            "127.0.0.1:0",
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("the server starts");

    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("the server's stdout"))
        .read_line(&mut line)
        .expect("the startup line");
    // `control-viz: listening on http://127.0.0.1:<port>/  (player: ...)`
    let rest = line
        .split("listening on http://")
        .nth(1)
        .unwrap_or_else(|| panic!("the startup line names the address: {line:?}"));
    let addr = rest
        .split('/')
        .next()
        .expect("the address ends at a slash")
        .to_string();
    (child, addr)
}

#[test]
fn the_route_tables_answers_are_the_expected_bytes() {
    let (viz, models, simrec) = fixture("routes");
    let missing = simrec.join("not_written.jsonl");

    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "/",
            expected("200 OK", Some("text/html; charset=utf-8"), PAGE),
        ),
        (
            "/lib/three.module.js",
            expected("200 OK", Some("text/javascript; charset=utf-8"), LIB),
        ),
        (
            "/models/unitree_g1/meshes/pelvis.STL",
            expected("200 OK", Some("application/octet-stream"), MESH),
        ),
        (
            "/simrec/g1_stand.jsonl",
            expected("200 OK", Some("application/json"), RECORDING),
        ),
        // the recording the player was asked for has not been written yet: a different 404 from a
        // route the table does not have, and it names the path so the reader knows which root moved
        (
            "/simrec/not_written.jsonl",
            expected(
                "404 Not Found",
                None,
                format!("missing: {}", missing.display()).as_bytes(),
            ),
        ),
        (
            "/nope",
            expected("404 Not Found", None, b"not found: /nope"),
        ),
        (
            "/../etc/passwd",
            expected("404 Not Found", None, b"not found: /../etc/passwd"),
        ),
    ];

    let (mut child, addr) = spawn(&viz, &models, &simrec, cases.len());
    // every answer is collected before anything is asserted, so a failing case cannot leave the
    // server waiting on a request this test no longer sends
    let got: Vec<(String, Vec<u8>)> = cases
        .iter()
        .map(|(target, _)| (target.to_string(), request(&addr, target)))
        .collect();
    child.wait().expect("the server stops after --max-requests");

    for ((target, want), (_, got)) in cases.iter().zip(&got) {
        assert_eq!(
            got,
            want,
            "{target}: expected\n{}\ngot\n{}",
            String::from_utf8_lossy(want),
            String::from_utf8_lossy(got)
        );
    }
}

#[test]
fn poke_is_served_from_the_same_page_directory() {
    let (viz, models, simrec) = fixture("poke");
    let (mut child, addr) = spawn(&viz, &models, &simrec, 1);
    let got = request(&addr, "/poke.html");
    child.wait().expect("the server stops after --max-requests");
    assert_eq!(
        got,
        expected("200 OK", Some("text/html; charset=utf-8"), POKE),
        "the force-injection page is one of the pages this crate ships"
    );
}

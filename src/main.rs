
use control_viz::{handle, Roots, DEFAULT_ADDR};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

const USAGE: &str = "\
usage: control-viz [options]

  --viz <dir>         the player page directory                  [<crate>/viz]
  --models <dir>      the mesh directory `/models/` names under  [../control-model/models]
  --simrec <dir>      the recordings directory                   [/tmp/simrec]
  --addr <addr>       the address to serve on                    [127.0.0.1:8321]
  --max-requests <n>  stop after n requests (0 = until killed)   [0]
  -h, --help          this text

Serves the scrub player and its data, read-only, one request per connection.";

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sibling = manifest
        .parent()
        .expect("the crate directory has a parent")
        .to_path_buf();
    let mut roots = Roots {
        viz: manifest.join("viz"),
        models: sibling.join("control-model").join("models"),
        simrec: PathBuf::from("/tmp/simrec"),
    };
    let mut addr = DEFAULT_ADDR.to_string();
    let mut max_requests = 0usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--viz" => roots.viz = PathBuf::from(value(&mut args, "--viz")),
            "--models" => roots.models = PathBuf::from(value(&mut args, "--models")),
            "--simrec" => roots.simrec = PathBuf::from(value(&mut args, "--simrec")),
            "--addr" => addr = value(&mut args, "--addr"),
            "--max-requests" => {
                max_requests = value(&mut args, "--max-requests")
                    .parse()
                    .unwrap_or_else(|_| fail("--max-requests takes a number"))
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            other => fail(&format!("unknown argument `{other}`")),
        }
    }

    for (flag, dir) in [
        ("--viz", &roots.viz),
        ("--models", &roots.models),
        ("--simrec", &roots.simrec),
    ] {
        if !dir.is_dir() {
            eprintln!(
                "control-viz: warning: {flag} {} is not a directory",
                dir.display()
            );
        }
    }

    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("control-viz: cannot listen on {addr}: {e}");
            std::process::exit(1);
        }
    };
    let bound = listener
        .local_addr()
        .expect("a bound listener has an address");
    println!("control-viz: listening on http://{bound}/  (player: http://{bound}/?rec=<name>)");

    let mut served = 0usize;
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let target = match read_request(&mut stream) {
            Some(target) => target,
            None => continue,
        };
        let _ = stream.write_all(&handle(&roots, &target).to_bytes());
        served += 1;
        if max_requests > 0 && served >= max_requests {
            eprintln!("control-viz: served {served} requests, stopping (--max-requests)");
            return;
        }
    }
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    args.next()
        .unwrap_or_else(|| fail(&format!("{flag} needs a value")))
}

fn fail(message: &str) -> ! {
    eprintln!("control-viz: {message}\n\n{USAGE}");
    std::process::exit(2)
}

fn read_request(stream: &mut TcpStream) -> Option<String> {
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !buf.contains(&b'\n') {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 64 * 1024 {
            return None;
        }
    }
    let line_end = buf.iter().position(|c| *c == b'\n').unwrap_or(buf.len());
    let line = String::from_utf8_lossy(&buf[..line_end]).to_string();
    let mut parts = line.trim_end_matches('\r').split(' ');
    parts.next();
    Some(parts.next().unwrap_or("").to_string())
}

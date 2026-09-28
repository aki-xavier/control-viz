
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_ADDR: &str = "127.0.0.1:8321";

pub struct Roots {
    pub viz: PathBuf,
    pub models: PathBuf,
    pub simrec: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    Route,
    Traversal,
}

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

fn not_found(body: String) -> Response {
    Response {
        status: 404,
        ctype: None,
        body: body.into_bytes(),
    }
}

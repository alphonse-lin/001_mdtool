//! `mdtool://` 自定义协议：/app/* 为嵌入的前端资源，/doc/* 为文档目录下的文件，/abs/* 为绝对路径文件。

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};
use wry::http::{header::CONTENT_TYPE, Request, Response};

static ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets");

#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    App(String),
    File(PathBuf),
    NotFound,
}

pub fn resolve(path: &str, doc_dir: &Path) -> Resolved {
    if let Some(rest) = path.strip_prefix("/app/") {
        Resolved::App(percent_decode(rest))
    } else if let Some(rest) = path.strip_prefix("/doc/") {
        Resolved::File(doc_dir.join(percent_decode(rest)))
    } else if let Some(rest) = path.strip_prefix("/abs/") {
        Resolved::File(PathBuf::from(percent_decode(rest)))
    } else {
        Resolved::NotFound
    }
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

pub fn handle(request: Request<Vec<u8>>, doc_dir: &Path) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path().to_owned();
    let body: Option<(Cow<'static, [u8]>, &'static str)> = match resolve(&path, doc_dir) {
        Resolved::App(name) => ASSETS
            .get_file(&name)
            .map(|f| (Cow::Borrowed(f.contents()), mime_for(&name))),
        Resolved::File(p) => std::fs::read(&p)
            .ok()
            .map(|data| (Cow::Owned(data), mime_for(&p.to_string_lossy()))),
        Resolved::NotFound => None,
    };
    match body {
        Some((data, mime)) => Response::builder().header(CONTENT_TYPE, mime).body(data),
        None => Response::builder().status(404).body(Cow::Borrowed(&b""[..])),
    }
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_app_assets() {
        assert_eq!(resolve("/app/index.html", Path::new("C:/d")), Resolved::App("index.html".into()));
    }

    #[test]
    fn resolves_doc_relative_files_with_percent_encoding() {
        let dir = Path::new("C:/d");
        assert_eq!(resolve("/doc/img/a%20b.png", dir), Resolved::File(dir.join("img/a b.png")));
        assert_eq!(resolve("/doc/%E5%9B%BE.png", dir), Resolved::File(dir.join("图.png")));
    }

    #[test]
    fn resolves_absolute_files() {
        assert_eq!(resolve("/abs/C:/x/y.png", Path::new("C:/d")), Resolved::File(PathBuf::from("C:/x/y.png")));
    }

    #[test]
    fn unknown_paths_are_not_found() {
        assert_eq!(resolve("/other", Path::new("C:/d")), Resolved::NotFound);
    }

    #[test]
    fn truncated_percent_sequences_are_kept() {
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn picks_mime_by_extension() {
        assert_eq!(mime_for("a.PNG"), "image/png");
        assert_eq!(mime_for("x.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime_for("f.woff2"), "font/woff2");
        assert_eq!(mime_for("noext"), "application/octet-stream");
    }

    #[test]
    fn index_html_is_embedded() {
        assert!(ASSETS.get_file("index.html").is_some());
    }
}

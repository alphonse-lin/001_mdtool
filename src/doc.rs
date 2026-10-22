//! 读取、解码、编码、保存 Markdown 文档。

use std::io;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Gbk,
}

#[derive(Debug)]
pub struct Decoded {
    pub text: String,
    pub encoding: Encoding,
    /// 既不是合法 UTF-8 也不是合法 GBK，按 UTF-8 有损解码。
    pub lossy: bool,
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn decode(bytes: &[u8]) -> Decoded {
    if let Some(rest) = bytes.strip_prefix(BOM) {
        return Decoded {
            text: String::from_utf8_lossy(rest).into_owned(),
            encoding: Encoding::Utf8Bom,
            lossy: std::str::from_utf8(rest).is_err(),
        };
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Decoded { text: text.to_owned(), encoding: Encoding::Utf8, lossy: false };
    }
    if let Some(text) = encoding_rs::GBK.decode_without_bom_handling_and_without_replacement(bytes) {
        return Decoded { text: text.into_owned(), encoding: Encoding::Gbk, lossy: false };
    }
    Decoded {
        text: String::from_utf8_lossy(bytes).into_owned(),
        encoding: Encoding::Utf8,
        lossy: true,
    }
}

pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => [BOM, text.as_bytes()].concat(),
        Encoding::Gbk => encoding_rs::GBK.encode(text).0.into_owned(),
    }
}

pub fn load(path: &Path) -> io::Result<Decoded> {
    Ok(decode(&std::fs::read(path)?))
}

/// 按指定编码保存，返回实际使用的编码：GBK 无法表示的字符（如 emoji）会让整篇改存为 UTF-8。
pub fn save(path: &Path, text: &str, encoding: Encoding) -> io::Result<Encoding> {
    let encoding = match encoding {
        Encoding::Gbk if encoding_rs::GBK.encode(text).2 => Encoding::Utf8,
        e => e,
    };
    std::fs::write(path, encode(text, encoding))?;
    Ok(encoding)
}

pub fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GBK_NIHAO: [u8; 4] = [0xC4, 0xE3, 0xBA, 0xC3];

    #[test]
    fn decodes_plain_utf8() {
        let d = decode(b"# hi");
        assert_eq!(d.text, "# hi");
        assert_eq!(d.encoding, Encoding::Utf8);
        assert!(!d.lossy);
    }

    #[test]
    fn decodes_utf8_with_bom() {
        let d = decode(b"\xEF\xBB\xBF# hi");
        assert_eq!(d.text, "# hi");
        assert_eq!(d.encoding, Encoding::Utf8Bom);
    }

    #[test]
    fn falls_back_to_gbk() {
        let d = decode(&GBK_NIHAO);
        assert_eq!(d.text, "你好");
        assert_eq!(d.encoding, Encoding::Gbk);
        assert!(!d.lossy);
    }

    #[test]
    fn invalid_bytes_are_lossy_utf8() {
        let d = decode(&[0xFF, 0xFF, 0xFF]);
        assert!(d.lossy);
        assert_eq!(d.encoding, Encoding::Utf8);
    }

    #[test]
    fn encodes_each_encoding() {
        assert_eq!(encode("你好", Encoding::Gbk), GBK_NIHAO);
        assert_eq!(encode("x", Encoding::Utf8Bom), b"\xEF\xBB\xBFx");
        assert_eq!(encode("x", Encoding::Utf8), b"x");
    }

    #[test]
    fn gbk_save_falls_back_to_utf8_for_unencodable_chars() {
        let path = std::env::temp_dir().join("mdtool_doc_emoji.md");
        assert_eq!(save(&path, "表情 😀", Encoding::Gbk).unwrap(), Encoding::Utf8);
        assert_eq!(std::fs::read(&path).unwrap(), "表情 😀".as_bytes());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = std::env::temp_dir().join("mdtool_doc_test.md");
        assert_eq!(save(&path, "中文 text", Encoding::Gbk).unwrap(), Encoding::Gbk);
        let d = load(&path).unwrap();
        assert_eq!(d.text, "中文 text");
        assert_eq!(d.encoding, Encoding::Gbk);
        assert!(mtime(&path).is_some());
        std::fs::remove_file(&path).unwrap();
    }
}

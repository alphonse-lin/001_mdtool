//! 界面语言：系统界面语言为中文时显示中文，否则显示英文；环境变量 MDTOOL_LANG=zh|en 可强制指定。

use std::sync::OnceLock;

use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

/// LANGID 的低 10 位是主语言，0x04 为 LANG_CHINESE（简体、繁体都算）。
const LANG_CHINESE: u16 = 0x04;

pub fn detect(env_override: Option<&str>, ui_langid: u16) -> Lang {
    let forced = env_override.map(str::to_ascii_lowercase);
    match forced.as_deref() {
        Some(s) if s.starts_with("zh") => Lang::Zh,
        Some(s) if s.starts_with("en") => Lang::En,
        _ if ui_langid & 0x3ff == LANG_CHINESE => Lang::Zh,
        _ => Lang::En,
    }
}

pub fn lang() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| {
        let ui = unsafe { GetUserDefaultUILanguage() };
        detect(std::env::var("MDTOOL_LANG").ok().as_deref(), ui)
    })
}

/// 传给前端的语言代码。
pub fn code() -> &'static str {
    match lang() {
        Lang::Zh => "zh",
        Lang::En => "en",
    }
}

/// 按当前语言二选一。
pub fn tr<'a>(zh: &'a str, en: &'a str) -> &'a str {
    match lang() {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZH_CN: u16 = 0x0804;
    const ZH_TW: u16 = 0x0404;
    const EN_US: u16 = 0x0409;
    const JA_JP: u16 = 0x0411;

    #[test]
    fn chinese_ui_languages_map_to_zh() {
        assert_eq!(detect(None, ZH_CN), Lang::Zh);
        assert_eq!(detect(None, ZH_TW), Lang::Zh);
    }

    #[test]
    fn other_ui_languages_map_to_en() {
        assert_eq!(detect(None, EN_US), Lang::En);
        assert_eq!(detect(None, JA_JP), Lang::En);
    }

    #[test]
    fn env_override_wins() {
        assert_eq!(detect(Some("en"), ZH_CN), Lang::En);
        assert_eq!(detect(Some("zh-CN"), EN_US), Lang::Zh);
        assert_eq!(detect(Some("EN"), ZH_CN), Lang::En);
    }

    #[test]
    fn unknown_override_falls_back_to_system() {
        assert_eq!(detect(Some("fr"), ZH_CN), Lang::Zh);
        assert_eq!(detect(Some(""), EN_US), Lang::En);
    }
}

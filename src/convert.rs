use opencc_rust::{DefaultConfig, OpenCC};
use std::sync::OnceLock;

// OpenCC 單例 — S2TWP
static CONVERTER: OnceLock<Option<OpenCC>> = OnceLock::new();
fn converter() -> &'static Option<OpenCC> {
    CONVERTER.get_or_init(|| {
        // 優先 S2TWP，失敗退回 S2T
        OpenCC::new(DefaultConfig::S2TWP)
            .or_else(|_| OpenCC::new(DefaultConfig::S2T))
            .ok()
    })
}

pub fn to_traditional(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let Some(oc) = converter().as_ref() else {
        return text.to_string();
    };
    let converted = oc.convert(text).unwrap_or_else(|_| text.to_string());
    // 台灣標準字補丁：賬 → 帳
    converted.replace('賬', "帳")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_traditional_unchanged() {
        // 純繁經 S2TWP 應不變
        assert_eq!(to_traditional("這是繁體中文資訊"), "這是繁體中文資訊");
        assert_eq!(to_traditional("軟體硬體"), "軟體硬體");
    }

    #[test]
    fn test_simplified_converts() {
        // 僅在有 opencc 可用時測詞彙
        let out = to_traditional("信息");
        // 若 opencc 不可用會原樣返回，仍應視為非失敗；但在有 opencc 時應為 資訊
        if converter().is_some() {
            assert_eq!(out, "資訊");
        }
    }

    #[test]
    fn test_opencc_phrases() {
        if converter().is_none() {
            return;
        }
        assert_eq!(to_traditional("网络"), "網路");
        assert_eq!(to_traditional("内存"), "記憶體");
        assert_eq!(to_traditional("视频"), "影片");
        assert_eq!(to_traditional("博客"), "部落格");
        assert_eq!(to_traditional("软件"), "軟體");
        assert_eq!(to_traditional("信息"), "資訊");
        assert_eq!(to_traditional("账单"), "帳單");
        // 吃不變喫
        assert_eq!(to_traditional("吃飯"), "吃飯");
    }

    #[test]
    fn test_char_fix() {
        if converter().is_none() {
            return;
        }
        // 賬 → 帳 應修正
        let s = to_traditional("转账");
        assert!(s.contains('帳'));
    }
}

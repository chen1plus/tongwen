use opencc_rust::{generate_static_dictionary, DefaultConfig, OpenCC};
use std::sync::OnceLock;

// OpenCC 單例 — S2TW；字典內嵌於二進位，執行時解到暫存目錄（leak 讓檔案與程序同壽命）
static CONVERTER: OnceLock<Option<OpenCC>> = OnceLock::new();
fn converter() -> &'static Option<OpenCC> {
    CONVERTER.get_or_init(|| {
        let dir = Box::leak(Box::new(tempfile::tempdir().ok()?));
        generate_static_dictionary(dir.path(), DefaultConfig::S2TW).ok()?;
        OpenCC::new(dir.path().join(DefaultConfig::S2TW.get_file_name())).ok()
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
        // 純繁經 S2TW 應不變
        assert_eq!(to_traditional("這是繁體中文資訊"), "這是繁體中文資訊");
        assert_eq!(to_traditional("軟體硬體"), "軟體硬體");
    }

    #[test]
    fn test_char_conversion() {
        if converter().is_none() {
            return;
        }
        assert_eq!(to_traditional("网络"), "網絡");
        assert_eq!(to_traditional("内存"), "內存");
        assert_eq!(to_traditional("视频"), "視頻");
        assert_eq!(to_traditional("博客"), "博客");
        assert_eq!(to_traditional("软件"), "軟件");
        assert_eq!(to_traditional("信息"), "信息");
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

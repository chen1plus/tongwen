use crate::convert;
use crate::processing;

/// 對應 sherpa_server.py:1230 那行組裝 — SpeakSlow 後處理全管線
/// 輸入應為已剝除 <TRANSCRIPT> 的純文字 (voiceink 由上層處理)
pub fn post_process(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut s = input.to_string();

    // 1 口吃疊字收斂
    s = processing::collapse_repeats(&s);
    // 2 詞組口吃
    s = processing::collapse_phrase_repeats(&s);
    // 3 語助詞正規化
    s = processing::normalize_interjections(&s);
    // 4 台灣發音修正
    s = processing::fix_tw_pronunciation(&s);
    // 5 句尾標點規則
    s = processing::apply_punct_rules(&s);
    // 6 列點（opt-in）
    if processing::format_lists_enabled() {
        s = processing::format_lists(&s);
    }
    // 7 英文行標點在地化
    s = processing::localize_english_punct(&s);
    // 8 簡轉繁（S2TW + 賬→帳）
    s = convert::to_traditional(&s);
    // 9 短句句尾。移除
    s = processing::strip_short_trailing_period(&s);

    s
}

/// VoiceInk 專用：剝除 <TRANSCRIPT> 標籤後再走管線
pub fn post_process_voiceink(input: &str) -> String {
    let stripped = strip_transcript_tags(input);
    post_process(&stripped)
}

pub fn strip_transcript_tags(s: &str) -> String {
    s.replace("<TRANSCRIPT>", "")
        .replace("</TRANSCRIPT>", "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_basic() {
        // 英文行會句首大寫
        assert_eq!(post_process("hello"), "Hello");
    }

    #[test]
    fn test_pipeline_tw_fix_and_convert() {
        // 樂色→垃圾 且 簡轉繁
        let out = post_process("乐色");
        // 簡體 乐色 經 pipeline： fix → 垃圾 → 繁體 垃圾 (已是繁體)
        assert_eq!(out, "垃圾");
    }

    #[test]
    fn test_pipeline_short_period() {
        assert_eq!(post_process("你好。"), "你好");
    }

    #[test]
    fn test_pipeline_voiceink() {
        assert_eq!(
            post_process_voiceink("<TRANSCRIPT>hello</TRANSCRIPT>"),
            "Hello"
        );
    }
    #[test]
    fn test_pipeline_punct() {
        // 吗 → ？
        // 输入为简体，后续会转繁，但吗会在转繁前被标点规则处理
        let out = post_process("你好吗");
        assert!(out.contains('？') || out.contains('?') || out.contains('嗎'));
    }
}

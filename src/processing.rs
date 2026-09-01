use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

// ---------- helpers ----------

fn is_cjk(ch: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&ch)
}

// ---------- 1. collapse_repeats ----------

fn valid_redup_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let raw = "爸爸 妈妈 媽媽 爹爹 哥哥 姐姐 弟弟 妹妹 爷爷 爺爺 奶奶 公公 婆婆 叔叔 婶婶 嬸嬸 伯伯 姑姑 舅舅 姨姨 宝宝 寶寶 乖乖 囡囡 妞妞 弟弟 哥哥 \
        看看 想想 试试 試試 走走 说说 說說 讲讲 講講 聊聊 玩玩 等等 找找 问问 問問 摸摸 抱抱 亲亲 親親 拍拍 数数 數數 闻闻 聞聞 尝尝 嚐嚐 写写 寫寫 读读 讀讀 算算 比比 量量 翻翻 查查 学学 學學 练练 練練 唱唱 跳跳 笑笑 猜猜 瞧瞧 望望 听听 聽聽 坐坐 站站 歇歇 动动 動動 转转 轉轉 晃晃 逛逛 试试 摇摇 搖搖 \
        慢慢 快快 刚刚 剛剛 常常 偏偏 渐渐 漸漸 轻轻 輕輕 重重 默默 悄悄 纷纷 紛紛 久久 早早 迟迟 遲遲 连连 連連 频频 頻頻 屡屡 屢屢 苦苦 深深 浅浅 淺淺 远远 遠遠 近近 团团 團團 牢牢 死死 紧紧 緊緊 松松 鬆鬆 稳稳 穩穩 偷偷 暗暗 明明 空空 满满 滿滿 处处 處處 时时 時時 步步 层层 層層 点点 點點 滴滴 一一 \
        个个 個個 条条 條條 件件 种种 種種 样样 樣樣 天天 年年 月月 日日 夜夜 人人 家家 户户 戶戶 村村 区区 區區 场场 場場 \
        好好 多多 少少 大大 小小 长长 長長 短短 胖胖 瘦瘦 圆圆 圓圓 扁扁 红红 紅紅 绿绿 綠綠 蓝蓝 藍藍 黄黄 黃黃 黑黑 白白 亮亮 甜甜 酸酸 辣辣 咸咸 鹹鹹 香香 臭臭 暖暖 凉凉 涼涼 热热 熱熱 冷冷 软软 軟軟 硬硬 厚厚 薄薄 嫩嫩 脆脆 高高 低低 矮矮 满满 \
        谢谢 謝謝 拜拜 嗯嗯 哈哈 呵呵 嘿嘿 嘻嘻 哼哼 喵喵 汪汪 咚咚 叮叮 啦啦 唉唉 哎哎 \
        呀呀 啊啊 哇哇 哦哦 喔喔 嗚嗚 噢噢 咦咦";
        raw.split_whitespace().map(|s| s.to_string()).collect()
    })
}

pub fn collapse_repeats(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    // 切成 run-length
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut runs: Vec<(char, usize)> = Vec::new();
    let mut i = 0;
    while i < n {
        let ch = chars[i];
        let mut j = i + 1;
        while j < n && chars[j] == ch {
            j += 1;
        }
        runs.push((ch, j - i));
        i = j;
    }

    let valid = valid_redup_set();
    let mut out = String::with_capacity(text.len());
    for k in 0..runs.len() {
        let (ch, run) = runs[k];
        if run >= 2 && is_cjk(ch) {
            let prev_dup = k > 0
                && runs[k - 1].1 >= 2
                && runs[k - 1].0 != ch
                && is_cjk(runs[k - 1].0);
            let next_dup = k + 1 < runs.len()
                && runs[k + 1].1 >= 2
                && runs[k + 1].0 != ch
                && is_cjk(runs[k + 1].0);
            let doubled = format!("{}{}", ch, ch);
            if valid.contains(&doubled) || prev_dup || next_dup {
                out.push(ch);
                out.push(ch);
            } else {
                out.push(ch);
            }
        } else {
            for _ in 0..run {
                out.push(ch);
            }
        }
    }
    out
}

// ---------- 2. collapse_phrase_repeats ----------

static FILLER_WORDS: &[&str] = &[
    "其實", "然後", "就是", "那個", "這個", "反正", "所以", "可是", "但是", "不過", "而且", "對啊",
    "對對", "那就", "之類",
];

fn collapse_repeated_phrase(text: &str, _phrase_len: usize, min_repeats: usize) -> String {
    // 手寫：尋找 2~4 字詞重複 3+ 次的 pattern，收成 1 次
    // 僅處理 CJK 字元
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        let mut matched = false;
        // 嘗試 len = 2..=4
        for l in 2..=4 {
            if i + l * min_repeats > n {
                continue;
            }
            // 檢查 phrase 是否全為 CJK (一-鿿)
            let phrase: Vec<char> = chars[i..i + l].to_vec();
            if !phrase.iter().all(|&c| is_cjk(c)) {
                continue;
            }
            // 檢查接下來是否重複
            let mut repeats = 1;
            let mut pos = i + l;
            while pos + l <= n && chars[pos..pos + l] == phrase[..] {
                repeats += 1;
                pos += l;
            }
            if repeats >= min_repeats {
                // 匹配成功，只保留一次
                for &c in &phrase {
                    out.push(c);
                }
                i = pos;
                matched = true;
                break;
            }
        }
        if !matched {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

pub fn collapse_phrase_repeats(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    // 1) 任意 2~4 字詞重複 3+ 次 → 1 次
    let mut s = collapse_repeated_phrase(text, 2, 3);
    // 上述函式已處理 2~4，若要確保該邏輯覆蓋所有，需對剩餘做最長匹配
    // 但我們已在函式內嘗試所有長度，所以只需呼叫一次即可
    // 為避免之前寫法中 phrase_len 參數被忽略，上面直接內部循環 2..4

    // 為確保完整，實際上 collapse_repeated_phrase 已經做了 2..4 的嘗試，所以 s 已是處理後
    // 但若有多處不相連的重複，需要整串掃描，已完成

    // 2) 發語詞重複 2+ 次 → 1 次
    // 手寫：對每個 filler word，把連續重複壓成一個
    for &w in FILLER_WORDS {
        let w_chars: Vec<char> = w.chars().collect();
        let w_len = w_chars.len();
        if w_len == 0 {
            continue;
        }
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let mut out = String::new();
        let mut i = 0;
        while i < n {
            // 檢查是否以 w 開頭
            if i + w_len <= n && chars[i..i + w_len] == w_chars[..] {
                // 計算連續重複次數
                let mut repeats = 0;
                let mut pos = i;
                while pos + w_len <= n && chars[pos..pos + w_len] == w_chars[..] {
                    repeats += 1;
                    pos += w_len;
                }
                if repeats >= 2 {
                    out.push_str(w);
                    i = pos;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        s = out;
    }
    s
}

// ---------- 3. normalize_interjections ----------

pub fn normalize_interjections(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    // 哎/誒 → 欸； 呃 移除
    let s = text.replace('哎', "欸").replace('誒', "欸");
    s.replace('呃', "")
}

// ---------- 4. fix_tw_pronunciation ----------

pub fn fix_tw_pronunciation(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    // 規則：
    // (?<![音娛娱快可歡欢享玩行作康安喜逸苦])[樂乐]色 → 垃圾
    // (?<!勾)勒色 → 垃圾
    // 手寫：掃描，找到「樂色/乐色/勒色」，檢查前一字是否在禁制集合
    let forbidden1: HashSet<char> = [
        '音', '娛', '娱', '快', '可', '歡', '欢', '享', '玩', '行', '作', '康', '安', '喜', '逸', '苦',
    ]
    .into_iter()
    .collect();

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if i + 1 < n && chars[i + 1] == '色' {
            let cur = chars[i];
            if cur == '樂' || cur == '乐' {
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                let forbidden = prev.map(|c| forbidden1.contains(&c)).unwrap_or(false);
                if !forbidden {
                    out.push_str("垃圾");
                    i += 2;
                    continue;
                }
            } else if cur == '勒' {
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                if prev != Some('勾') {
                    out.push_str("垃圾");
                    i += 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// ---------- 5. apply_punct_rules ----------

static PARTICLE_PUNCT: OnceLock<std::collections::HashMap<char, char>> = OnceLock::new();
fn particle_map() -> &'static std::collections::HashMap<char, char> {
    PARTICLE_PUNCT.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        m.insert('吗', '？');
        m.insert('嗎', '？');
        m.insert('呢', '？');
        m.insert('啊', '！');
        m.insert('呀', '！');
        m.insert('啦', '！');
        m.insert('哇', '！');
        m.insert('喔', '！');
        m.insert('哦', '！');
        m.insert('噢', '！');
        m.insert('耶', '！');
        m.insert('欸', '！');
        m
    })
}

static PUNCT_RULES: OnceLock<Vec<(char, Vec<&'static str>)>> = OnceLock::new();
fn punct_rules() -> &'static Vec<(char, Vec<&'static str>)> {
    PUNCT_RULES.get_or_init(|| {
        vec![
            (
                '？',
                vec![
                    "什么啊", "怎么啊", "为什么啊", "可以吗", "好吗", "是不是", "对不对", "行不行", "好不好",
                    "要不要", "有没有", "是吗", "对吗", "能不能",
                ],
            ),
            (
                '！',
                vec![
                    "怎样啦", "这样啦", "干嘛啦", "什么啦", "这样啊", "欸呀呀", "太棒了", "好厉害", "真的假的",
                    "不会吧", "天啊", "我的天",
                ],
            ),
        ]
    })
}

static PUNCT_COMPILED: OnceLock<Vec<(Regex, Regex, String)>> = OnceLock::new();
fn punct_compiled() -> &'static Vec<(Regex, Regex, String)> {
    PUNCT_COMPILED.get_or_init(|| {
        let mut v = Vec::new();
        for (mark, phrases) in punct_rules() {
            for p in phrases {
                let pat1 = format!(r"{}[。，！？]", regex::escape(p));
                let pat2 = format!(r"{}$", regex::escape(p));
                v.push((
                    Regex::new(&pat1).unwrap(),
                    Regex::new(&pat2).unwrap(),
                    format!("{}{}", p, mark),
                ));
            }
        }
        v
    })
}

pub fn apply_punct_rules(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut s = text.to_string();
    let particle_class = "吗嗎呢啊呀啦哇喔哦噢耶欸";
    let p_map = particle_map();

    // 1) 句末單字語助詞 + 句末標點 → 換標點
    static RE1: OnceLock<Regex> = OnceLock::new();
    let re1 = RE1.get_or_init(|| {
        let pat = format!(r"([{}])([。！？])", regex::escape(particle_class));
        Regex::new(&pat).unwrap()
    });
    s = re1
        .replace_all(&s, |caps: &regex::Captures| {
            let ch = caps[1].chars().next().unwrap();
            let punct = p_map.get(&ch).copied().unwrap_or('？');
            format!("{}{}", ch, punct)
        })
        .into_owned();

    // 句末單字語助詞在字串結尾、沒有標點 → 補上
    static RE2: OnceLock<Regex> = OnceLock::new();
    let re2 = RE2.get_or_init(|| {
        let pat = format!(r"([{}])$", regex::escape(particle_class));
        Regex::new(&pat).unwrap()
    });
    s = re2
        .replace_all(&s, |caps: &regex::Captures| {
            let ch = caps[1].chars().next().unwrap();
            let punct = p_map.get(&ch).copied().unwrap();
            format!("{}{}", ch, punct)
        })
        .into_owned();

    // 2) 多字片語補充（預編譯，避免每請求編譯 50 次 Regex）
    for (re_punct, re_eos, replacement) in punct_compiled() {
        s = re_punct.replace_all(&s, replacement.as_str()).into_owned();
        s = re_eos.replace_all(&s, replacement.as_str()).into_owned();
    }

    // 3) 台灣語尾「吼」→ 哈 還原
    // (?<!哈)哈(?=[。！？，、]|$)
    s = {
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < n {
            if chars[i] == '哈' {
                let prev_is_ha = i > 0 && chars[i - 1] == '哈';
                let next = if i + 1 < n { Some(chars[i + 1]) } else { None };
                let next_is_boundary = match next {
                    None => true,
                    Some(c) => matches!(c, '。' | '！' | '？' | '，' | '、'),
                };
                if !prev_is_ha && next_is_boundary {
                    out.push('吼');
                    i += 1;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    };

    // 4) 句尾「好」→「吼」保守子集
    // (?<=[了啊嘛耶喔欸吧呢哦對])好(?=[。！？，、]|$)
    s = {
        let triggers: HashSet<char> = ['了', '啊', '嘛', '耶', '喔', '欸', '吧', '呢', '哦', '對']
            .into_iter()
            .collect();
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < n {
            if chars[i] == '好' {
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                let prev_ok = prev.map(|c| triggers.contains(&c)).unwrap_or(false);
                let next = if i + 1 < n { Some(chars[i + 1]) } else { None };
                let next_is_boundary = match next {
                    None => true,
                    Some(c) => matches!(c, '。' | '！' | '？' | '，' | '、'),
                };
                if prev_ok && next_is_boundary {
                    out.push('吼');
                    i += 1;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    };

    s
}

// ---------- 6. format_lists ----------

pub fn format_lists(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    // 需在 pipeline 層決定是否啟用，此處僅做轉換
    // 實現對照 Python 的 format_lists
    static RE_MARKER: OnceLock<Regex> = OnceLock::new();
    let re_marker = RE_MARKER.get_or_init(|| Regex::new(r"第([一二三四五六七八九十兩两])").unwrap());

    let mut markers: Vec<(usize, usize, String)> = Vec::new(); // (byte_start, byte_end, ord)
    for m in re_marker.find_iter(text) {
        let full = m.as_str();
        let ord = m.as_str().chars().nth(1).unwrap().to_string(); // 第X 的 X
        let start = m.start();
        let end = m.end();
        // 檢查後一個字是否為 次名件
        let after = text[end..].chars().next();
        if let Some(c) = after {
            if matches!(c, '次' | '名' | '件') {
                continue;
            }
        }
        // full 包含 "第X"
        markers.push((start, end, ord));
        let _ = full;
    }

    if markers.len() < 2 || markers[0].2 != "一" {
        return text.to_string();
    }

    let intro_end = markers[0].0;
    let intro_raw = text[..intro_end].trim();
    let intro = intro_raw.trim_end_matches(|c| matches!(c, '，' | ',' | '、' | '：' | ':' | '；' | ';' | '。' | '　' | ' '));

    let mut items: Vec<String> = Vec::new();
    for idx in 0..markers.len() {
        let (_, end, _) = markers[idx];
        let next_start = if idx + 1 < markers.len() {
            markers[idx + 1].0
        } else {
            text.len()
        };
        let mut seg = text[end..next_start].to_string();

        // 去掉項目開頭的量詞/名詞/連接詞
        static RE_PREFIX: OnceLock<Regex> = OnceLock::new();
        let re_prefix = RE_PREFIX.get_or_init(|| {
            Regex::new(
                r"^(?:[個个件位種种項项條条張张份步點点] ?)?(?:事情|事兒|事儿|事|東西|东西|原因|問題|问题|地方|方面|步驟|步骤)?(?:是|為|为|要|就是)?[，,、：:。　\s]*",
            )
            .unwrap()
        });
        seg = re_prefix.replace(&seg, "").into_owned();
        let seg = seg.trim().trim_end_matches(|c| matches!(c, '。' | '，' | ',' | '、' | '；' | ';' | '　' | ' ')).to_string();
        if !seg.is_empty() {
            items.push(seg);
        }
    }

    if items.len() < 2 {
        return text.to_string();
    }

    let body = items
        .into_iter()
        .enumerate()
        .map(|(i, it)| format!("{}. {}", i + 1, it))
        .collect::<Vec<_>>()
        .join("\n");

    if intro.is_empty() {
        body
    } else {
        format!("{}：\n{}", intro, body)
    }
}

pub fn format_lists_enabled() -> bool {
    // 由環境變數控制，與 Python 的 _FORMAT_LISTS_ENABLED 對應
    // 用於 pipeline 層判斷
    std::env::var("TONGWEN_LISTS").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
}

// ---------- 7. localize_english_punct ----------

static EN_FILLER_CJK: &[char] = &['啊', '嗯', '呃', '哦', '喔', '欸', '唉', '誒', '呀', '嘛'];

fn is_english_dominant(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let cjk: Vec<char> = line.chars().filter(|c| is_cjk(*c)).collect();
    let ascii_letters = line.chars().filter(|c| c.is_ascii_alphabetic()).count();
    if cjk.is_empty() {
        return ascii_letters > 0;
    }
    ascii_letters >= 12
        && cjk.len() <= 1 + ascii_letters / 15
        && cjk.iter().all(|c| EN_FILLER_CJK.contains(c))
}

pub fn localize_english_punct(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out_lines = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() || !is_english_dominant(line) {
            out_lines.push(line.to_string());
            continue;
        }
        let mut l = line.to_string();
        // filler → 空格
        static RE_FILLER: OnceLock<Regex> = OnceLock::new();
        let re_filler = RE_FILLER.get_or_init(|| {
            let pat = format!("[{}]+", EN_FILLER_CJK.iter().collect::<String>());
            Regex::new(&pat).unwrap()
        });
        l = re_filler.replace_all(&l, " ").into_owned();

        for (a, b) in [
            ('，', ", "),
            ('。', ". "),
            ('？', "? "),
            ('！', "! "),
            ('：', ": "),
            ('；', "; "),
            ('、', ", "),
        ] {
            l = l.replace(a, b);
        }
        static RE_SPACE_PUNCT: OnceLock<Regex> = OnceLock::new();
        let re_sp = RE_SPACE_PUNCT.get_or_init(|| Regex::new(r"\s+([,.?!:;])").unwrap());
        l = re_sp.replace_all(&l, "$1").into_owned();

        static RE_MULTI_SPACE: OnceLock<Regex> = OnceLock::new();
        let re_ms = RE_MULTI_SPACE.get_or_init(|| Regex::new(r"\s{2,}").unwrap());
        l = re_ms.replace_all(&l, " ").into_owned();
        l = l.trim().to_string();

        static RE_CAP: OnceLock<Regex> = OnceLock::new();
        let re_cap = RE_CAP.get_or_init(|| Regex::new(r"(^|[.?!]\s+)([a-z])").unwrap());
        l = re_cap
            .replace_all(&l, |caps: &regex::Captures| {
                let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let letter = caps.get(2).unwrap().as_str();
                format!("{}{}", prefix, letter.to_uppercase())
            })
            .into_owned();

        static RE_I: OnceLock<Regex> = OnceLock::new();
        let re_i = RE_I.get_or_init(|| Regex::new(r"\bi\b").unwrap());
        l = re_i.replace_all(&l, "I").into_owned();

        out_lines.push(l);
    }
    out_lines.join("\n")
}

// ---------- 8. strip_short_trailing_period ----------

pub fn strip_short_trailing_period(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.trim().contains('\n') {
        return text.to_string();
    }
    let t = text.trim_end();
    if t.ends_with('。') {
        let body = &t[..t.len() - '。'.len_utf8()];
        if !body.contains('。') && !body.contains('！') && !body.contains('？') && body.trim().chars().count() <= 5 {
            return body.to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapse_repeats() {
        assert_eq!(collapse_repeats("我我我"), "我");
        assert_eq!(collapse_repeats("慢慢"), "慢慢");
        assert_eq!(collapse_repeats("謝謝"), "謝謝");
        assert_eq!(collapse_repeats("吃吃喝喝"), "吃吃喝喝");
        assert_eq!(collapse_repeats("吃吃牛肉"), "吃牛肉");
        assert_eq!(collapse_repeats("我我我愛你"), "我愛你");
        assert_eq!(collapse_repeats("hello"), "hello");
    }

    #[test]
    fn test_collapse_phrase_repeats() {
        assert_eq!(collapse_phrase_repeats("其實其實其實"), "其實");
        assert_eq!(collapse_phrase_repeats("然後然後"), "然後");
        assert_eq!(collapse_phrase_repeats("研究研究"), "研究研究"); // 剛好2次非 filler 不動
        assert_eq!(collapse_phrase_repeats("其實其實其實其實"), "其實");
    }

    #[test]
    fn test_normalize_interjections() {
        assert_eq!(normalize_interjections("哎呀"), "欸呀");
        assert_eq!(normalize_interjections("誒呀"), "欸呀");
        assert_eq!(normalize_interjections("呃呃"), "");
        assert_eq!(normalize_interjections("欸呃"), "欸");
    }

    #[test]
    fn test_fix_tw_pronunciation() {
        assert_eq!(fix_tw_pronunciation("樂色"), "垃圾");
        assert_eq!(fix_tw_pronunciation("勒色"), "垃圾");
        assert_eq!(fix_tw_pronunciation("音樂"), "音樂");
        assert_eq!(fix_tw_pronunciation("色彩"), "色彩");
        assert_eq!(fix_tw_pronunciation("音樂色彩"), "音樂色彩");
        assert_eq!(fix_tw_pronunciation("可樂色素"), "可樂色素");
    }

    #[test]
    fn test_apply_punct_rules() {
        assert_eq!(apply_punct_rules("吗。"), "吗？");
        assert_eq!(apply_punct_rules("啦"), "啦！");
        assert_eq!(apply_punct_rules("真的假的"), "真的假的！");
        assert_eq!(apply_punct_rules("對吼"), "對吼");
        assert_eq!(apply_punct_rules("搞好了好"), "搞好了吼");
        assert_eq!(apply_punct_rules("哈哈"), "哈哈"); // 不動
        assert_eq!(apply_punct_rules("哈。"), "吼。"); // 單哈→吼
    }

    #[test]
    fn test_format_lists() {
        let input = "今天要做三件事第一是買菜第二是煮飯第三是洗碗";
        let out = format_lists(input);
        assert!(out.contains("1. 買菜"));
        assert!(out.contains("2. 煮飯"));
        assert!(out.contains("3. 洗碗"));
        // 不符合條件不轉
        assert_eq!(format_lists("第一次見面"), "第一次見面");
    }

    #[test]
    fn test_localize_english_punct() {
        assert_eq!(
            localize_english_punct("hello，how are you？"),
            "Hello, how are you?"
        );
        assert_eq!(
            localize_english_punct("hello，world。 i am here。"),
            "Hello, world. I am here."
        );
        // 中英混雜不動
        assert_eq!(
            localize_english_punct("你好 hello，world？"),
            "你好 hello，world？"
        );
    }

    #[test]
    fn test_strip_short_trailing_period() {
        assert_eq!(strip_short_trailing_period("你好。"), "你好");
        assert_eq!(strip_short_trailing_period("你好嗎？"), "你好嗎？");
        assert_eq!(strip_short_trailing_period("這是一個很長的句子。"), "這是一個很長的句子。");
        assert_eq!(strip_short_trailing_period("第一行。\n第二行。"), "第一行。\n第二行。");
        assert_eq!(strip_short_trailing_period("hi。"), "hi");
    }
}

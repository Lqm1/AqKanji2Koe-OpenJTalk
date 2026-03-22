use crate::mora::mora_katakana_to_hiragana;
use crate::phoneme::{
    devoiced_kana, devoiced_roman, get_doubling_consonant, katakana_mora_to_roman,
};

/// 音声記号列の出力形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// AquesTalk 標準のひらがな表記。
    Kana,
    /// AquesTalk pico 準拠の ASCII ローマ字表記。
    Roman,
}

/// アクセント句の後ろに置く区切り記号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    /// `。` または `.` に対応する文末記号。
    Period,
    /// `？` または `?` に対応する疑問文末記号。
    Question,
    /// `、` または `,` に対応する区切り記号。
    Comma,
    /// `;` に対応する区切り記号。
    Semicolon,
    /// `/` に対応する通常のアクセント句区切り。
    Slash,
    /// `+` に対応する副次アクセント句区切り。
    Plus,
}

impl Delimiter {
    fn as_str(self, format: OutputFormat) -> &'static str {
        match format {
            OutputFormat::Kana => match self {
                Self::Period => "。",
                Self::Question => "？",
                Self::Comma => "、",
                Self::Semicolon => ";",
                Self::Slash => "/",
                Self::Plus => "+",
            },
            OutputFormat::Roman => match self {
                Self::Period => ".",
                Self::Question => "?",
                Self::Comma => ",",
                Self::Semicolon => ";",
                Self::Slash => "/",
                Self::Plus => "+",
            },
        }
    }

    pub fn kana_str(self) -> &'static str {
        self.as_str(OutputFormat::Kana)
    }

    pub fn roman_str(self) -> &'static str {
        self.as_str(OutputFormat::Roman)
    }

    /// ポーズを伴う区切りかどうかを返す。
    pub fn is_before_pause(self) -> bool {
        matches!(self, Self::Period | Self::Question | Self::Comma)
    }

    fn is_sentence_end(self) -> bool {
        matches!(self, Self::Period | Self::Question)
    }
}

/// NJD ノードから抽出した最小限のデータ。
#[derive(Debug, Clone)]
pub struct NodeData {
    /// 記号判定に使う元テキスト。
    pub original: String,
    /// `(カタカナモーラ文字列, 有声フラグ)` の列。
    ///
    /// `is_voiced = false` はそのモーラが無声化されていることを示す。
    pub pron_moras: Vec<(String, bool)>,
    /// アクセント位置。`0` は平板。
    pub accent: usize,
    /// 前のノードと同じアクセント句に連結するかどうか。
    pub chain_with_prev: bool,
    /// 発音が空かどうか。
    pub is_pron_empty: bool,
    /// 読点・句点として扱うノードかどうか。
    pub is_touten: bool,
    /// 疑問符として扱うノードかどうか。
    pub is_question: bool,
}

#[derive(Debug, Clone)]
struct AccentPhrase {
    moras: Vec<(String, bool)>,
    accent: usize,
}

enum Item {
    Phrase(AccentPhrase),
    Delim(Delimiter),
}

fn flush_phrase(items: &mut Vec<Item>, moras: &mut Vec<(String, bool)>, accent: usize) {
    if moras.is_empty() {
        return;
    }

    items.push(Item::Phrase(AccentPhrase {
        moras: std::mem::take(moras),
        accent,
    }));
}

fn build_items(nodes: &[NodeData]) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let mut cur_moras: Vec<(String, bool)> = Vec::new();
    let mut cur_accent: usize = 0;

    for node in nodes {
        if let Some(delim) = detect_delimiter(node) {
            flush_phrase(&mut items, &mut cur_moras, cur_accent);
            items.push(Item::Delim(delim));
            continue;
        }

        if node.is_pron_empty || node.pron_moras.is_empty() {
            continue;
        }

        if !node.chain_with_prev || cur_moras.is_empty() {
            flush_phrase(&mut items, &mut cur_moras, cur_accent);
            cur_accent = node.accent;
        }
        cur_moras.extend(node.pron_moras.iter().cloned());
    }

    flush_phrase(&mut items, &mut cur_moras, cur_accent);
    items
}

fn pair_phrases(items: Vec<Item>) -> Vec<(AccentPhrase, Delimiter)> {
    let mut result: Vec<(AccentPhrase, Delimiter)> = Vec::new();
    let mut pending: Option<AccentPhrase> = None;

    for item in items {
        match item {
            Item::Phrase(p) => {
                if let Some(prev) = pending.replace(p) {
                    result.push((prev, Delimiter::Slash));
                }
            }
            Item::Delim(d) => {
                if let Some(p) = pending.take() {
                    result.push((p, d));
                } else if matches!(d, Delimiter::Period | Delimiter::Question) {
                    // 連続した文末記号は、直前の句の区切りとして扱う。
                    if let Some(last) = result.last_mut() {
                        last.1 = d;
                    }
                }
            }
        }
    }

    if let Some(p) = pending {
        result.push((p, Delimiter::Period));
    }

    // 最後の句は常に文末記号で閉じる。
    if result
        .last()
        .map(|(_, d)| !d.is_sentence_end())
        .unwrap_or(false)
    {
        if let Some(last) = result.last_mut() {
            last.1 = Delimiter::Period;
        }
    }

    result
}

fn detect_delimiter(node: &NodeData) -> Option<Delimiter> {
    if node.is_question {
        return Some(Delimiter::Question);
    }

    let s = node.original.as_str();

    if node.is_touten || node.is_pron_empty {
        return delimiter_from_pronless_symbol(s)
            .or_else(|| node.is_touten.then_some(Delimiter::Comma));
    }

    if node.pron_moras.is_empty() {
        return delimiter_from_ascii_symbol(s);
    }

    None
}

fn delimiter_from_pronless_symbol(symbol: &str) -> Option<Delimiter> {
    match symbol {
        "。" | "." | "．" => Some(Delimiter::Period),
        "、" | "，" => Some(Delimiter::Comma),
        "？" | "?" => Some(Delimiter::Question),
        "！" | "!" => Some(Delimiter::Period),
        "…" | "⋯" => Some(Delimiter::Period),
        "　" | " " => Some(Delimiter::Comma),
        _ => None,
    }
}

fn delimiter_from_ascii_symbol(symbol: &str) -> Option<Delimiter> {
    match symbol {
        "." => Some(Delimiter::Period),
        "," => Some(Delimiter::Comma),
        "?" => Some(Delimiter::Question),
        "!" => Some(Delimiter::Period),
        " " => Some(Delimiter::Comma),
        _ => None,
    }
}

fn format_phrase_kana(phrase: &AccentPhrase) -> String {
    let mut out = String::new();

    for (i, (mora, is_voiced)) in phrase.moras.iter().enumerate() {
        let mora_idx = i + 1;

        if mora == "ッ" {
            out.push('っ');
        } else if !is_voiced {
            if let Some(d) = devoiced_kana(mora) {
                out.push_str(&d);
            } else {
                out.push_str(&mora_katakana_to_hiragana(mora));
            }
        } else {
            out.push_str(&mora_katakana_to_hiragana(mora));
        }

        if phrase.accent > 0 && mora_idx == phrase.accent {
            out.push('\'');
        }
    }

    out
}

fn format_phrase_roman(phrase: &AccentPhrase) -> String {
    let mut out = String::new();
    let mut pending_double: Option<char> = None;

    for (i, (mora, is_voiced)) in phrase.moras.iter().enumerate() {
        let mora_idx = i + 1;

        if mora == "ッ" {
            let double_char = if i + 1 < phrase.moras.len() {
                let (next_mora, next_voiced) = &phrase.moras[i + 1];
                let next_roman = if !next_voiced {
                    devoiced_roman(next_mora).unwrap_or_else(|| katakana_mora_to_roman(next_mora))
                } else {
                    katakana_mora_to_roman(next_mora)
                };
                get_doubling_consonant(next_roman.strip_prefix('_').unwrap_or(next_roman))
            } else {
                None
            };

            if let Some(c) = double_char {
                pending_double = Some(c);
            } else {
                if let Some(c) = pending_double.take() {
                    out.push(c);
                }
                out.push_str("xtu");
            }

            if phrase.accent > 0 && mora_idx == phrase.accent {
                out.push('\'');
            }
            continue;
        }

        if let Some(c) = pending_double.take() {
            out.push(c);
        }

        let roman = if !is_voiced {
            devoiced_roman(mora).unwrap_or_else(|| katakana_mora_to_roman(mora))
        } else {
            katakana_mora_to_roman(mora)
        };

        out.push_str(roman);

        if phrase.accent > 0 && mora_idx == phrase.accent {
            out.push('\'');
        }
    }

    out
}

fn format_phrase(phrase: &AccentPhrase, format: OutputFormat) -> String {
    match format {
        OutputFormat::Kana => format_phrase_kana(phrase),
        OutputFormat::Roman => format_phrase_roman(phrase),
    }
}

/// NJD ノード列から音声記号列を構築する。
pub fn nodes_to_phoneme(nodes: &[NodeData], format: OutputFormat) -> String {
    let items = build_items(nodes);
    let pairs = pair_phrases(items);

    if pairs.is_empty() {
        return match format {
            OutputFormat::Kana => "。".to_string(),
            OutputFormat::Roman => ".".to_string(),
        };
    }

    let mut out = String::new();

    for (phrase, delim) in &pairs {
        let phrase_str = format_phrase(phrase, format);
        out.push_str(&phrase_str);
        out.push_str(delim.as_str(format));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{nodes_to_phoneme, NodeData, OutputFormat};

    fn spoken_node(moras: &[(&str, bool)], accent: usize, chain_with_prev: bool) -> NodeData {
        NodeData {
            original: moras.iter().map(|(mora, _)| *mora).collect(),
            pron_moras: moras
                .iter()
                .map(|(mora, is_voiced)| ((*mora).to_string(), *is_voiced))
                .collect(),
            accent,
            chain_with_prev,
            is_pron_empty: false,
            is_touten: false,
            is_question: false,
        }
    }

    fn ascii_symbol_node(symbol: &str) -> NodeData {
        NodeData {
            original: symbol.to_string(),
            pron_moras: Vec::new(),
            accent: 0,
            chain_with_prev: false,
            is_pron_empty: false,
            is_touten: false,
            is_question: false,
        }
    }

    #[test]
    fn empty_input_defaults_to_sentence_end() {
        assert_eq!(nodes_to_phoneme(&[], OutputFormat::Kana), "。");
        assert_eq!(nodes_to_phoneme(&[], OutputFormat::Roman), ".");
    }

    #[test]
    fn implicit_phrase_and_sentence_delimiters_are_added() {
        let nodes = [
            spoken_node(&[("カ", true)], 0, false),
            spoken_node(&[("キ", true)], 0, false),
        ];

        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Kana), "か/き。");
        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Roman), "ka/ki.");
    }

    #[test]
    fn ascii_punctuation_is_detected_without_pronunciation() {
        let nodes = [
            spoken_node(&[("コ", true), ("レ", true)], 0, false),
            ascii_symbol_node(","),
            spoken_node(&[("デ", true)], 1, false),
        ];

        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Kana), "これ、で'。");
        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Roman), "kore,de'.");
    }

    #[test]
    fn spaces_are_normalized_to_commas() {
        let nodes = [
            spoken_node(&[("コ", true), ("レ", true)], 0, false),
            ascii_symbol_node(" "),
            spoken_node(&[("デ", true)], 1, false),
        ];

        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Kana), "これ、で'。");
        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Roman), "kore,de'.");
    }

    #[test]
    fn devoiced_sokuon_uses_following_consonant_in_roman_output() {
        let nodes = [spoken_node(&[("ッ", true), ("キ", false)], 0, false)];

        assert_eq!(nodes_to_phoneme(&nodes, OutputFormat::Roman), "k_ki.");
    }
}

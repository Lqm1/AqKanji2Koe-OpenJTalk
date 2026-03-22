use crate::mora::mora_katakana_to_hiragana;
use crate::phoneme::{
    devoiced_kana, devoiced_roman, get_doubling_consonant, katakana_mora_to_roman,
};

// ── 出力形式 ────────────────────────────────────────────────────────────────

/// 音声記号列の出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// ひらがなかな記法（AquesTalk 標準）
    Kana,
    /// ASCII ローマ字記法（AquesTalk pico 準拠）
    Roman,
}

// ── 句切記号 ────────────────────────────────────────────────────────────────

/// アクセント句の後に続く区切り記号
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    /// `。` / `.` — 文末（ポーズあり）
    Period,
    /// `？` / `?` — 文末疑問形（ポーズあり）
    Question,
    /// `、` / `,` — 呼気段落境界（ポーズあり）
    Comma,
    /// `;` — 次句が高い音で始まる（ポーズなし）
    Semicolon,
    /// `/` — 通常のアクセント句区切り（ポーズなし）
    Slash,
    /// `+` — 副次アクセント句区切り（ポーズなし）
    Plus,
    /// 半角スペース — 呼気段落境界（ポーズあり）
    Space,
}

impl Delimiter {
    pub fn kana_str(self) -> &'static str {
        match self {
            Self::Period    => "。",
            Self::Question  => "？",
            Self::Comma     => "、",
            Self::Semicolon => ";",
            Self::Slash     => "/",
            Self::Plus      => "+",
            Self::Space     => " ",
        }
    }

    pub fn roman_str(self) -> &'static str {
        match self {
            Self::Period    => ".",
            Self::Question  => "?",
            Self::Comma     => ",",
            Self::Semicolon => ";",
            Self::Slash     => "/",
            Self::Plus      => "+",
            Self::Space     => " ",
        }
    }

    /// ポーズを伴う区切りか（無声化判定に使用）
    pub fn is_before_pause(self) -> bool {
        matches!(self, Self::Period | Self::Question | Self::Comma | Self::Space)
    }
}

// ── 内部データ構造 ───────────────────────────────────────────────────────────

/// NJD ノードから抽出した最小限のデータ
#[derive(Debug, Clone)]
pub struct NodeData {
    /// 元テキスト（記号判定用）
    pub original: String,
    /// (カタカナモーラ文字列, 有声フラグ) のリスト
    ///
    /// jpreprocess の `Pronunciation::moras()` から構築する。
    /// `is_voiced = false` はそのモーラが無声化されていることを示す。
    pub pron_moras: Vec<(String, bool)>,
    /// アクセント位置（0 = 平板、N = 第 N モーラ後に下降）
    pub accent: usize,
    /// 前のノードと同じアクセント句に連結するか
    pub chain_with_prev: bool,
    /// 発音が空（読点・句点などの無音ノード）
    pub is_pron_empty: bool,
    /// 読点/句点ノード
    pub is_touten: bool,
    /// 疑問符ノード
    pub is_question: bool,
}

/// 1 つのアクセント句
#[derive(Debug, Clone)]
struct AccentPhrase {
    /// (カタカナモーラ文字列, 有声フラグ) のリスト
    moras: Vec<(String, bool)>,
    /// アクセント核の位置（0 = 平板）
    accent: usize,
}

/// ビルダー内部アイテム
enum Item {
    Phrase(AccentPhrase),
    Delim(Delimiter),
}

// ── フレーズ構築 ─────────────────────────────────────────────────────────────

/// NJD ノード列をアクセント句と区切り記号のアイテム列に変換する
fn build_items(nodes: &[NodeData]) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let mut cur_moras: Vec<(String, bool)> = Vec::new();
    let mut cur_accent: usize = 0;

    let flush = |items: &mut Vec<Item>, moras: &mut Vec<(String, bool)>, accent: usize| {
        if !moras.is_empty() {
            items.push(Item::Phrase(AccentPhrase {
                moras: moras.drain(..).collect(),
                accent,
            }));
        }
    };

    for node in nodes {
        // ── 区切り記号ノード ──────────────────────────────────────────────
        if let Some(delim) = detect_delimiter(node) {
            flush(&mut items, &mut cur_moras, cur_accent);
            items.push(Item::Delim(delim));
            continue;
        }

        if node.is_pron_empty || node.pron_moras.is_empty() {
            // 発音なし・区切りでもないノード（記号等）はスキップ
            continue;
        }

        // ── 通常の発音ノード ──────────────────────────────────────────────
        if !node.chain_with_prev || cur_moras.is_empty() {
            // 新しいアクセント句の始まり
            flush(&mut items, &mut cur_moras, cur_accent);
            cur_accent = node.accent;
        }
        // このノードのモーラを追加
        cur_moras.extend(node.pron_moras.iter().cloned());
    }

    // 残ったモーラをフラッシュ
    flush(&mut items, &mut cur_moras, cur_accent);
    items
}

/// アイテム列をアクセント句と後続区切り記号のペアに変換する
///
/// - フレーズの後に区切り記号がない場合は Slash をデフォルトで使う
/// - 最後のフレーズに区切り記号がない場合は Period を付加する
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
                    // 直前にフレーズがない文末記号 → 前のフレーズの区切りを更新
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

    // 末尾が Period/Question でない場合は Period を付加
    if result
        .last()
        .map(|(_, d)| !matches!(d, Delimiter::Period | Delimiter::Question))
        .unwrap_or(false)
    {
        if let Some(last) = result.last_mut() {
            last.1 = Delimiter::Period;
        }
    }

    result
}

/// ノードデータから区切り記号を検出する
fn detect_delimiter(node: &NodeData) -> Option<Delimiter> {
    if node.is_question {
        return Some(Delimiter::Question);
    }

    let s = node.original.as_str();

    if node.is_touten || node.is_pron_empty {
        return match s {
            "。" | "." | "．" => Some(Delimiter::Period),
            "、" | "，"       => Some(Delimiter::Comma),
            "？" | "?"        => Some(Delimiter::Question),
            "！" | "!"        => Some(Delimiter::Period),
            "…" | "⋯"        => Some(Delimiter::Period),
            "　"              => Some(Delimiter::Space),
            " "               => Some(Delimiter::Space),
            _ => {
                if node.is_touten {
                    Some(Delimiter::Comma)
                } else {
                    None
                }
            }
        };
    }

    // 半角記号が入力に含まれていた場合も検出
    if node.pron_moras.is_empty() {
        return match s {
            "." => Some(Delimiter::Period),
            "," => Some(Delimiter::Comma),
            "?" => Some(Delimiter::Question),
            "!" => Some(Delimiter::Period),
            _ => None,
        };
    }

    None
}

// ── フォーマット ──────────────────────────────────────────────────────────────

/// アクセント句をかな記法でフォーマットする
///
/// - 有声モーラ → ひらがな
/// - 無声化モーラ → `_カタカナ` (spec §無声化手動指定)
/// - アクセント核の直後に `'`
fn format_phrase_kana(phrase: &AccentPhrase) -> String {
    let mut out = String::new();

    for (i, (mora, is_voiced)) in phrase.moras.iter().enumerate() {
        let mora_idx = i + 1; // 1-indexed

        if mora == "ッ" {
            out.push('っ');
        } else if !is_voiced {
            // 無声化: _カタカナ表記
            if let Some(d) = devoiced_kana(mora) {
                out.push_str(&d);
            } else {
                // 仕様にない無声化モーラはひらがなで出力
                out.push_str(&mora_katakana_to_hiragana(mora));
            }
        } else {
            out.push_str(&mora_katakana_to_hiragana(mora));
        }

        // アクセント核マーカー
        if phrase.accent > 0 && mora_idx == phrase.accent {
            out.push('\'');
        }
    }

    out
}

/// アクセント句をローマ字記法でフォーマットする
///
/// - ッ → 後続モーラの語頭子音を重ねる（子音連続）、末尾は xtu
/// - 無声化モーラ → `_roman`
/// - アクセント核の直後に `'`
fn format_phrase_roman(phrase: &AccentPhrase) -> String {
    let mut out = String::new();
    // ッ から来た保留子音（次のモーラの頭に付ける）
    let mut pending_double: Option<char> = None;

    for (i, (mora, is_voiced)) in phrase.moras.iter().enumerate() {
        let mora_idx = i + 1; // 1-indexed

        if mora == "ッ" {
            // 後続モーラの語頭子音で連続させる
            let double_char = if i + 1 < phrase.moras.len() {
                let (next_mora, next_voiced) = &phrase.moras[i + 1];
                let next_roman = if !next_voiced {
                    devoiced_roman(next_mora).unwrap_or_else(|| katakana_mora_to_roman(next_mora))
                } else {
                    katakana_mora_to_roman(next_mora)
                };
                get_doubling_consonant(next_roman)
            } else {
                None
            };

            if let Some(c) = double_char {
                pending_double = Some(c);
            } else {
                // 末尾ッ または母音始まり後続 → xtu
                if let Some(c) = pending_double.take() {
                    out.push(c);
                }
                out.push_str("xtu");
            }

            // ッ もモーラなのでアクセントマーカーを確認
            if phrase.accent > 0 && mora_idx == phrase.accent {
                out.push('\'');
            }
            continue;
        }

        // 保留していた二重子音を出力
        if let Some(c) = pending_double.take() {
            out.push(c);
        }

        // モーラのローマ字（有声/無声で分岐）
        let roman = if !is_voiced {
            devoiced_roman(mora).unwrap_or_else(|| katakana_mora_to_roman(mora))
        } else {
            katakana_mora_to_roman(mora)
        };

        out.push_str(roman);

        // アクセント核マーカー
        if phrase.accent > 0 && mora_idx == phrase.accent {
            out.push('\'');
        }
    }

    out
}

// ── 公開 API ─────────────────────────────────────────────────────────────────

/// NJD ノードデータから音声記号列を構築する
pub fn nodes_to_phoneme(nodes: &[NodeData], format: OutputFormat) -> String {
    let items = build_items(nodes);
    let pairs = pair_phrases(items);

    if pairs.is_empty() {
        return match format {
            OutputFormat::Kana  => "。".to_string(),
            OutputFormat::Roman => ".".to_string(),
        };
    }

    let mut out = String::new();

    for (phrase, delim) in &pairs {
        let phrase_str = match format {
            OutputFormat::Kana  => format_phrase_kana(phrase),
            OutputFormat::Roman => format_phrase_roman(phrase),
        };
        out.push_str(&phrase_str);
        out.push_str(match format {
            OutputFormat::Kana  => delim.kana_str(),
            OutputFormat::Roman => delim.roman_str(),
        });
    }

    out
}

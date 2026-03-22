/// カタカナ 1 文字をひらがなに変換する。
///
/// ー (U+30FC) および変換対象外の文字はそのまま返す。
#[inline]
pub fn katakana_char_to_hiragana(c: char) -> char {
    let code = c as u32;
    // カタカナ U+30A1〜U+30F6 はひらがな U+3041〜U+3096 に対応 (オフセット -0x60)
    if (0x30A1..=0x30F6).contains(&code) {
        char::from_u32(code - 0x60).unwrap_or(c)
    } else {
        c
    }
}

/// カタカナのモーラ文字列をひらがなに変換する。
///
/// ー はそのまま残る（ひらがな表記でも ー を使う）。
pub fn mora_katakana_to_hiragana(mora: &str) -> String {
    mora.chars().map(katakana_char_to_hiragana).collect()
}

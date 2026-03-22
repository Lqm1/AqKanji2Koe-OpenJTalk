//! # aqkanji2koe
//!
//! 漢字かな交じりテキストを AquesTalk 用音声記号列に変換するライブラリ。
//!
//! 内部で [jpreprocess](https://docs.rs/jpreprocess) (OpenJTalk の Rust 実装) と
//! NAIST-JDic を使用して形態素解析・アクセント推定を行う。
//!
//! ## 出力形式
//!
//! - **かな記法** — ひらがな + 半角記号 (AquesTalk 標準入力形式、UTF-8)
//! - **ローマ字記法** — ASCII のみ (AquesTalk pico 準拠)
//!
//! ## 基本的な使い方
//!
//! ```rust
//! use aqkanji2koe::AqKanji2Koe;
//!
//! let converter = AqKanji2Koe::new().expect("初期化失敗");
//!
//! let kana   = converter.convert("日本語のテキスト").unwrap();
//! let roman  = converter.convert_roman("日本語のテキスト").unwrap();
//!
//! println!("かな: {kana}");
//! println!("ローマ字: {roman}");
//! ```

pub mod converter;
pub mod error;
pub mod mora;
pub mod phoneme;

pub use converter::OutputFormat;
pub use error::{Error, Result};

use converter::{nodes_to_phoneme, NodeData};
use jpreprocess::kind::JPreprocessDictionaryKind;
use jpreprocess::{JPreprocess, SystemDictionaryConfig};

// ── AqKanji2Koe 本体 ────────────────────────────────────────────────────────

/// 漢字かな交じりテキスト → AquesTalk 音声記号列 変換器
///
/// jpreprocess (OpenJTalk) と NAIST-JDic をバンドルして使用する。
/// スレッド間で共有可能 (`Send + Sync`)。
pub struct AqKanji2Koe {
    /// テキスト → NodeData 列 の処理関数（型消去済み）
    process: Box<dyn Fn(&str) -> Result<Vec<NodeData>> + Send + Sync>,
}

impl AqKanji2Koe {
    /// バンドル済み NAIST-JDic を使って変換器を初期化する。
    ///
    /// # Errors
    ///
    /// 辞書の読み込みに失敗した場合に [`Error::Init`] を返す。
    pub fn new() -> Result<Self> {
        let system = SystemDictionaryConfig::Bundled(JPreprocessDictionaryKind::NaistJdic)
            .load()
            .map_err(|e| Error::Init(e.to_string()))?;

        let jp = JPreprocess::with_dictionaries(system, None);

        let process = Box::new(move |text: &str| -> Result<Vec<NodeData>> {
            // text_to_njd は形態素解析のみ。
            // preprocess() を呼ぶことでアクセント句連結・無声化フラグが確定する。
            let mut njd = jp
                .text_to_njd(text)
                .map_err(|e| Error::Processing(e.to_string()))?;
            njd.preprocess();

            let nodes = njd
                .nodes
                .iter()
                .map(|node| {
                    let pron = node.get_pron();
                    // pron.moras() の各 Mora から (カタカナ文字列, is_voiced) を抽出する。
                    // Mora::Display は is_voiced=false のとき末尾に "'" を追加するため、
                    // strip_suffix で除去してカタカナのみを取り出す。
                    // jpreprocess の Mora::Display は無声化時に U+2019 RIGHT SINGLE
                    // QUOTATION MARK (') を末尾に付加する。これを取り除いてカタカナだけを得る。
                    let pron_moras: Vec<(String, bool)> = pron
                        .moras()
                        .iter()
                        .map(|m| {
                            let s = m.to_string();
                            // U+2019 RIGHT SINGLE QUOTATION MARK を除去
                            let kata = s.strip_suffix('\u{2019}').unwrap_or(&s).to_string();
                            (kata, m.is_voiced)
                        })
                        .collect();
                    NodeData {
                        original: node.get_string().to_string(),
                        pron_moras,
                        accent: pron.accent(),
                        chain_with_prev: node.get_chain_flag().unwrap_or(false),
                        is_pron_empty: pron.is_empty(),
                        is_touten: pron.is_touten(),
                        is_question: pron.is_question(),
                    }
                })
                .collect();

            Ok(nodes)
        });

        Ok(Self { process })
    }

    // ── かな出力 ───────────────────────────────────────────────────────────

    /// 漢字かな交じりテキストを **かな音声記号列** (UTF-8) に変換する。
    ///
    /// 出力例: `"これわ/おんせ'ーきごーです。"`
    ///
    /// # Errors
    ///
    /// jpreprocess の処理に失敗した場合に [`Error::Processing`] を返す。
    pub fn convert(&self, text: &str) -> Result<String> {
        let nodes = (self.process)(text)?;
        Ok(nodes_to_phoneme(&nodes, OutputFormat::Kana))
    }

    // ── ローマ字出力 ───────────────────────────────────────────────────────

    /// 漢字かな交じりテキストを **ローマ字音声記号列** (ASCII) に変換する。
    ///
    /// 出力例: `"korewa/onse'-kigo-desu."`
    ///
    /// # Errors
    ///
    /// jpreprocess の処理に失敗した場合に [`Error::Processing`] を返す。
    pub fn convert_roman(&self, text: &str) -> Result<String> {
        let nodes = (self.process)(text)?;
        Ok(nodes_to_phoneme(&nodes, OutputFormat::Roman))
    }
}

impl std::fmt::Debug for AqKanji2Koe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AqKanji2Koe").finish_non_exhaustive()
    }
}

# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Commands

```sh
# ビルド（全クレート）
cargo build

# リリースビルド
cargo build --release

# CLIで動作確認
cargo run --bin aqkanji2koe -- "テキスト"
cargo run --bin aqkanji2koe -- "テキスト" --roman

# C ABIライブラリのみビルド
cargo build -p aqkanji2koe-capi

# テスト
cargo test

# 特定クレートのテスト
cargo test -p aqkanji2koe

# 単一テスト
cargo test -p aqkanji2koe <test_name>

# lintと警告チェック
cargo clippy
```

## アーキテクチャ

Cargo ワークスペース構成。3つのクレートがある:

- **ルート** (`src/main.rs`) — CLI バイナリ `aqkanji2koe`。`--roman` フラグで出力形式切替、引数なし時は stdin から1行ずつ処理する。
- **`crates/aqkanji2koe`** — コアライブラリ（rlib）。公開 API は `AqKanji2Koe` 構造体のみ。
- **`crates/aqkanji2koe-capi`** — C ABI ラッパー（cdylib + staticlib）。`OnceLock<AqKanji2Koe>` によるプロセス内シングルトン。

### コアライブラリの処理フロー

```
入力テキスト
  └─ jpreprocess::text_to_njd() → NJD ノード列
  └─ njd.preprocess()           ← 必須: アクセント句連結・無声化フラグを確定させる
  └─ NodeData 列に変換           （モーラ文字列 + is_voiced フラグ）
  └─ build_items()              → AccentPhrase / Delimiter のアイテム列
  └─ pair_phrases()             → (AccentPhrase, Delimiter) のペア列
  └─ format_phrase_kana/roman() → 文字列
  └─ 連結して出力
```

**`lib.rs`** — `AqKanji2Koe` の初期化と NJD → `NodeData` 変換。`JPreprocess<D>` のジェネリック型を型消去するため、処理関数を `Box<dyn Fn(&str) -> Result<Vec<NodeData>> + Send + Sync>` として保持する。

**`converter.rs`** — `NodeData` → 音声記号列の全変換ロジック。`NodeData.pron_moras` は `Vec<(String, bool)>`（カタカナモーラ文字列, is_voiced）。

**`phoneme.rs`** — カタカナ → ローマ字変換テーブル（`katakana_mora_to_roman`）と無声化記号変換（`devoiced_roman`, `devoiced_kana`）。

**`mora.rs`** — カタカナ→ひらがな変換（`mora_katakana_to_hiragana`）。

### jpreprocess の重要な注意点

- `text_to_njd()` の後に必ず `njd.preprocess()` を呼ぶこと。呼ばないとアクセント句の chain_flag が全て `None` のままになり、句が正しくまとまらない。
- `Mora::Display`（`to_string()`）は無声化モーラの末尾に U+2019 RIGHT SINGLE QUOTATION MARK (`'`) を付加する。ASCII アポストロフィ U+0027 ではない。モーラ文字列を取り出す際は `strip_suffix('\u{2019}')` で除去する（`lib.rs` 参照）。
- 無声化フラグは jpreprocess の `Mora.is_voiced` を直接使用する（ルールベースの `compute_devoicing` は使わない）。

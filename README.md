# AqKanji2Koe-OpenJTalk

漢字かな交じりテキストを AquesTalk 用音声記号列に変換する Rust ライブラリ・CLIツール・C ABI ライブラリです。
日本語解析に [jpreprocess](https://github.com/jpreprocess/jpreprocess)（OpenJTalk の Rust 実装）と NAIST-JDic を使用します。

## 免責事項

本プロジェクトは、株式会社アクエストが提供する商用ライブラリ **AqKanji2Koe** および **AquesTalk** とは無関係の独立した再実装です。

- 本プロジェクトのコードは、公開されている仕様書（音声記号列仕様）を参考に独自に実装したものであり、オリジナルのバイナリ・ソースコード・非公開情報は一切使用していません。
- AqKanji2Koe・AquesTalk の商標および著作権は株式会社アクエストに帰属します。
- 本プロジェクトは株式会社アクエストによって承認・保証されたものではありません。
- 本ソフトウェアの使用によって生じたいかなる損害についても、作者は責任を負いません。

AquesTalk を実際の製品で利用する場合は、[株式会社アクエストの公式サイト](https://www.a-quest.com/)にてライセンスを取得してください。

## 出力形式

| 形式 | 例 | 説明 |
|---|---|---|
| かな記法 | `にほんごの/て'_キすとで_ス。` | ひらがな + 半角記号（AquesTalk 標準入力形式、UTF-8） |
| ローマ字記法 | `nihonngono/te'_kisutode_su.` | ASCII のみ（AquesTalk pico 準拠） |


## プロジェクト構成

```
.
├── src/main.rs                     # CLIバイナリ
├── crates/
    ├── aqkanji2koe/                # コアライブラリ (rlib)
    └── aqkanji2koe-capi/           # C ABI ラッパー (cdylib + staticlib)
```

## CLIツール

### ビルド

```sh
cargo build --release
```

### 使い方

```sh
# 引数から変換（かな記法）
aqkanji2koe "日本語のテキストです。"
# => にほんごの/て'_キすとで_ス。

# ローマ字記法
aqkanji2koe --roman "日本語のテキストです。"
# => nihonngono/te'_kisutode_su.

# stdin から1行ずつ変換
echo "今日は晴れています。" | aqkanji2koe
```

## Rustライブラリ (`aqkanji2koe`)

`Cargo.toml` に追加:

```toml
[dependencies]
aqkanji2koe = { path = "path/to/crates/aqkanji2koe" }
```

### 使い方

```rust
use aqkanji2koe::AqKanji2Koe;

let converter = AqKanji2Koe::new().expect("初期化失敗");

// かな記法
let kana = converter.convert("日本語のテキストです。").unwrap();
println!("{kana}"); // にほんごの/て'_キすとで_ス。

// ローマ字記法
let roman = converter.convert_roman("日本語のテキストです。").unwrap();
println!("{roman}"); // nihonngono/te'_kisutode_su.
```

`AqKanji2Koe` は `Send + Sync` なので `Arc` 等で複数スレッドから共有できます。

## C ABI ライブラリ (`aqkanji2koe-capi`)

### 配布

GitHub のタグ `v*` でリリースすると、GitHub Releases に OS ごとの C ABI バンドルを自動添付します。
バンドルにはライブラリ本体、C ヘッダ `include/aqkanji2koe.h`、`README.md` が含まれます。

現状の自動配布対象:

- Windows 64-bit (`x86_64-pc-windows-msvc`)
- Windows 32-bit (`i686-pc-windows-msvc`)
- Windows ARM64 (`aarch64-pc-windows-msvc`)
- Linux 64-bit (`x86_64-unknown-linux-gnu`)
- Linux ARM64 (`aarch64-unknown-linux-gnu`)
- macOS Apple Silicon (`aarch64-apple-darwin`)
- Android (`armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`)
- iOS (`aqkanji2koe.xcframework`: device arm64 + simulator arm64/x86_64)

補足:

- iOS は通常の `.dylib` 配布ではなく、Xcode に取り込みやすい `XCFramework` を配布します。
- Android は ABI ごとの `libaqkanji2koe.so` をまとめた `tar.gz` を配布します。
- bare-metal 向けの `no_std` 組み込みターゲットは非対応です。`jpreprocess` と辞書データを使う都合上、OS と標準ライブラリを前提にしています。

例:

- `aqkanji2koe-capi-v0.1.0-x86_64-pc-windows-msvc.tar.gz`
- `aqkanji2koe-capi-v0.1.0-i686-pc-windows-msvc.tar.gz`
- `aqkanji2koe-capi-v0.1.0-aarch64-pc-windows-msvc.tar.gz`
- `aqkanji2koe-capi-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- `aqkanji2koe-capi-v0.1.0-aarch64-unknown-linux-gnu.tar.gz`
- `aqkanji2koe-capi-v0.1.0-aarch64-apple-darwin.tar.gz`
- `aqkanji2koe-capi-v0.1.0-android.tar.gz`
- `aqkanji2koe-capi-v0.1.0-ios-xcframework.tar.gz`

### ビルド

```sh
# ネイティブターゲット
cargo build --release -p aqkanji2koe-capi

# 32ビット Windows (cross-compilation)
cargo build --release -p aqkanji2koe-capi --target i686-pc-windows-msvc
```

ビルド成果物:

| プラットフォーム | 動的ライブラリ | 静的ライブラリ |
|---|---|---|
| Windows | `aqkanji2koe.dll` (`aqkanji2koe.dll.lib` は import library) | `aqkanji2koe.lib` |
| Linux | `libaqkanji2koe.so` | `libaqkanji2koe.a` |
| macOS | `libaqkanji2koe.dylib` | `libaqkanji2koe.a` |

### API

```c
// 初期化（冪等、プロセス内で1回呼び出せばよい）
int aqk2k_create(void);

// 解放（現在は no-op、プロセス終了時に自動解放）
void aqk2k_release(void);

// かな音声記号列に変換（UTF-8入力・UTF-8出力）
int aqk2k_convert(const char *input_utf8, char *out_buf, int buf_size);

// ローマ字音声記号列に変換（UTF-8入力・ASCII出力）
int aqk2k_convert_roman(const char *input_utf8, char *out_buf, int buf_size);

// かな音声記号列に変換（UTF-16LE入力・UTF-8出力）
int aqk2k_convert_u16(const uint16_t *input_utf16, char *out_buf, int buf_size);

// ローマ字音声記号列に変換（UTF-16LE入力・ASCII出力）
int aqk2k_convert_roman_u16(const uint16_t *input_utf16, char *out_buf, int buf_size);
```

#### エラーコード

| コード | 内容 |
|---|---|
| `0` | 成功 |
| `1` | 引数エラー（NULLポインタ等） |
| `2` | 未初期化（`aqk2k_create` を呼んでいない） |
| `3` | バッファ不足 |
| `4` | 処理エラー |

#### 使用例（C）

```c
#include "aqkanji2koe.h"

int main(void) {
    if (aqk2k_create() != 0) return 1;

    char buf[512];
    int ret = aqk2k_convert("日本語のテキストです。", buf, sizeof(buf));
    if (ret == 0) printf("%s\n", buf);

    aqk2k_release();
    return 0;
}
```

## 動作要件

- Rust 1.75 以上
- NAIST-JDic は `jpreprocess` の `naist-jdic` feature でバンドル済み（別途インストール不要）

## ライセンス

MIT

//! # aqkanji2koe-capi
//!
//! `aqkanji2koe` の C ABI ラッパー。
//!
//! `.dll` / `.so` / `.dylib` および静的ライブラリとしてビルドされ、
//! C/C++ や他言語から `extern "C"` 関数として呼び出せる。
//!
//! ## ビルド成果物
//!
//! | プラットフォーム | cdylib | staticlib |
//! |---|---|---|
//! | Windows (32/64-bit) | `aqkanji2koe.dll` | `aqkanji2koe.lib` |
//! | Linux | `libaqkanji2koe.so` | `libaqkanji2koe.a` |
//! | macOS | `libaqkanji2koe.dylib` | `libaqkanji2koe.a` |
//!
//! ## エラーコード
//!
//! | コード | 内容 |
//! |---|---|
//! | 0 | 正常終了 |
//! | 1 | 引数エラー（NULL ポインタ等） |
//! | 2 | 初期化されていない |
//! | 3 | バッファ不足 |
//! | 4 | 処理エラー |

use aqkanji2koe::AqKanji2Koe;
use std::ffi::{CStr, c_char, c_int};
use std::sync::OnceLock;

// ── エラーコード ─────────────────────────────────────────────────────────────

const ERR_OK: c_int           = 0;
const ERR_INVALID_ARG: c_int  = 1;
const ERR_NOT_INIT: c_int     = 2;
const ERR_BUFFER_SMALL: c_int = 3;
const ERR_PROCESSING: c_int   = 4;

// ── グローバルインスタンス ───────────────────────────────────────────────────

/// プロセス内で共有するシングルトンインスタンス
static INSTANCE: OnceLock<AqKanji2Koe> = OnceLock::new();

// ── 初期化 / 解放 ────────────────────────────────────────────────────────────

/// 変換器を初期化する。
///
/// 既に初期化済みの場合は何もしない (冪等)。
///
/// # 戻り値
/// - `0`: 成功
/// - `4`: 初期化エラー
#[unsafe(no_mangle)]
pub extern "C" fn aqk2k_create() -> c_int {
    if INSTANCE.get().is_some() {
        return ERR_OK;
    }
    match AqKanji2Koe::new() {
        Ok(inst) => {
            let _ = INSTANCE.set(inst);
            ERR_OK
        }
        Err(_) => ERR_PROCESSING,
    }
}

/// 変換器を解放する (OnceLock ではドロップできないため現在は no-op)。
///
/// 互換性のために提供する。プロセス終了時に自動解放される。
#[unsafe(no_mangle)]
pub extern "C" fn aqk2k_release() {
    // OnceLock は Drop 不可のため、プロセス終了まで保持される
}

// ── 変換関数（UTF-8 入力・UTF-8 かな出力）───────────────────────────────────

/// 漢字かな交じり文（UTF-8）をかな音声記号列（UTF-8）に変換する。
///
/// # 引数
/// - `input_utf8`  : 入力文字列ポインタ（NULL 終端 UTF-8）
/// - `out_buf`     : 出力バッファポインタ（NULL 終端 UTF-8 が格納される）
/// - `buf_size`    : バッファのバイトサイズ
///
/// # 戻り値
/// - `0` : 成功
/// - `1` : 引数エラー（NULL ポインタ）
/// - `2` : 未初期化
/// - `3` : バッファ不足
/// - `4` : 処理エラー
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert(
    input_utf8: *const c_char,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    if input_utf8.is_null() || out_buf.is_null() || buf_size <= 0 {
        return ERR_INVALID_ARG;
    }

    let Some(inst) = INSTANCE.get() else {
        return ERR_NOT_INIT;
    };

    let text = unsafe {
        match CStr::from_ptr(input_utf8).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_INVALID_ARG,
        }
    };

    let result = match inst.convert(text) {
        Ok(s) => s,
        Err(_) => return ERR_PROCESSING,
    };

    write_cstr(out_buf, buf_size as usize, &result)
}

/// 漢字かな交じり文（UTF-8）をローマ字音声記号列（ASCII）に変換する。
///
/// # 引数 / 戻り値
/// [`aqk2k_convert`] と同様。出力は ASCII ローマ字音声記号列。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_roman(
    input_utf8: *const c_char,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    if input_utf8.is_null() || out_buf.is_null() || buf_size <= 0 {
        return ERR_INVALID_ARG;
    }

    let Some(inst) = INSTANCE.get() else {
        return ERR_NOT_INIT;
    };

    let text = unsafe {
        match CStr::from_ptr(input_utf8).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_INVALID_ARG,
        }
    };

    let result = match inst.convert_roman(text) {
        Ok(s) => s,
        Err(_) => return ERR_PROCESSING,
    };

    write_cstr(out_buf, buf_size as usize, &result)
}

// ── 変換関数（UTF-16 入力）──────────────────────────────────────────────────

/// 漢字かな交じり文（UTF-16LE）をかな音声記号列（UTF-8）に変換する。
///
/// # 引数
/// - `input_utf16` : 入力文字列ポインタ（NUL 終端 UTF-16LE ワード列）
/// - `out_buf`     : 出力バッファポインタ（UTF-8）
/// - `buf_size`    : バッファのバイトサイズ
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_u16(
    input_utf16: *const u16,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    if input_utf16.is_null() || out_buf.is_null() || buf_size <= 0 {
        return ERR_INVALID_ARG;
    }

    let Some(inst) = INSTANCE.get() else {
        return ERR_NOT_INIT;
    };

    let text = match utf16_ptr_to_string(input_utf16) {
        Some(s) => s,
        None => return ERR_INVALID_ARG,
    };

    let result = match inst.convert(&text) {
        Ok(s) => s,
        Err(_) => return ERR_PROCESSING,
    };

    write_cstr(out_buf, buf_size as usize, &result)
}

/// 漢字かな交じり文（UTF-16LE）をローマ字音声記号列（ASCII）に変換する。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_roman_u16(
    input_utf16: *const u16,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    if input_utf16.is_null() || out_buf.is_null() || buf_size <= 0 {
        return ERR_INVALID_ARG;
    }

    let Some(inst) = INSTANCE.get() else {
        return ERR_NOT_INIT;
    };

    let text = match utf16_ptr_to_string(input_utf16) {
        Some(s) => s,
        None => return ERR_INVALID_ARG,
    };

    let result = match inst.convert_roman(&text) {
        Ok(s) => s,
        Err(_) => return ERR_PROCESSING,
    };

    write_cstr(out_buf, buf_size as usize, &result)
}

// ── ヘルパー ─────────────────────────────────────────────────────────────────

/// 文字列を NUL 終端 C 文字列としてバッファに書き込む。
///
/// バッファが不足する場合は `ERR_BUFFER_SMALL` を返す。
unsafe fn write_cstr(buf: *mut c_char, size: usize, s: &str) -> c_int {
    let bytes = s.as_bytes();
    // NUL 終端分 +1 が必要
    if bytes.len() + 1 > size {
        return ERR_BUFFER_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0; // NUL 終端
    }
    ERR_OK
}

/// NUL 終端 UTF-16LE ワード列を Rust の `String` に変換する。
unsafe fn utf16_ptr_to_string(ptr: *const u16) -> Option<String> {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
        if len > 65536 {
            return None; // 安全のため上限を設ける
        }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16(slice).ok()
}

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
use std::ffi::{c_char, c_int, CStr};
use std::sync::OnceLock;

const ERR_OK: c_int = 0;
const ERR_INVALID_ARG: c_int = 1;
const ERR_NOT_INIT: c_int = 2;
const ERR_BUFFER_SMALL: c_int = 3;
const ERR_PROCESSING: c_int = 4;

static INSTANCE: OnceLock<AqKanji2Koe> = OnceLock::new();

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
///
/// # Safety
///
/// `input_utf8` は有効な NUL 終端 UTF-8 文字列を指し、`out_buf` は `buf_size`
/// バイト以上書き込み可能な有効バッファを指している必要がある。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert(
    input_utf8: *const c_char,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    let buf_size = match validate_conversion_args(input_utf8, out_buf, buf_size) {
        Ok(buf_size) => buf_size,
        Err(code) => return code,
    };

    unsafe {
        with_utf8_input(input_utf8, |text| {
            write_converted(out_buf, buf_size, |inst| inst.convert(text))
        })
    }
}

/// 漢字かな交じり文（UTF-8）をローマ字音声記号列（ASCII）に変換する。
///
/// # 引数 / 戻り値
/// [`aqk2k_convert`] と同様。出力は ASCII ローマ字音声記号列。
///
/// # Safety
///
/// `input_utf8` は有効な NUL 終端 UTF-8 文字列を指し、`out_buf` は `buf_size`
/// バイト以上書き込み可能な有効バッファを指している必要がある。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_roman(
    input_utf8: *const c_char,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    let buf_size = match validate_conversion_args(input_utf8, out_buf, buf_size) {
        Ok(buf_size) => buf_size,
        Err(code) => return code,
    };

    unsafe {
        with_utf8_input(input_utf8, |text| {
            write_converted(out_buf, buf_size, |inst| inst.convert_roman(text))
        })
    }
}

/// 漢字かな交じり文（UTF-16LE）をかな音声記号列（UTF-8）に変換する。
///
/// # 引数
/// - `input_utf16` : 入力文字列ポインタ（NUL 終端 UTF-16LE ワード列）
/// - `out_buf`     : 出力バッファポインタ（UTF-8）
/// - `buf_size`    : バッファのバイトサイズ
///
/// # Safety
///
/// `input_utf16` は有効な NUL 終端 UTF-16 ワード列を指し、`out_buf` は `buf_size`
/// バイト以上書き込み可能な有効バッファを指している必要がある。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_u16(
    input_utf16: *const u16,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    let buf_size = match validate_conversion_args(input_utf16, out_buf, buf_size) {
        Ok(buf_size) => buf_size,
        Err(code) => return code,
    };

    unsafe {
        with_utf16_input(input_utf16, |text| {
            write_converted(out_buf, buf_size, |inst| inst.convert(text))
        })
    }
}

/// 漢字かな交じり文（UTF-16LE）をローマ字音声記号列（ASCII）に変換する。
///
/// # Safety
///
/// `input_utf16` は有効な NUL 終端 UTF-16 ワード列を指し、`out_buf` は `buf_size`
/// バイト以上書き込み可能な有効バッファを指している必要がある。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aqk2k_convert_roman_u16(
    input_utf16: *const u16,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> c_int {
    let buf_size = match validate_conversion_args(input_utf16, out_buf, buf_size) {
        Ok(buf_size) => buf_size,
        Err(code) => return code,
    };

    unsafe {
        with_utf16_input(input_utf16, |text| {
            write_converted(out_buf, buf_size, |inst| inst.convert_roman(text))
        })
    }
}

fn validate_conversion_args<T>(
    input: *const T,
    out_buf: *mut c_char,
    buf_size: c_int,
) -> std::result::Result<usize, c_int> {
    if input.is_null() || out_buf.is_null() || buf_size <= 0 {
        Err(ERR_INVALID_ARG)
    } else {
        Ok(buf_size as usize)
    }
}

fn get_instance() -> std::result::Result<&'static AqKanji2Koe, c_int> {
    INSTANCE.get().ok_or(ERR_NOT_INIT)
}

unsafe fn with_utf8_input(input_utf8: *const c_char, f: impl FnOnce(&str) -> c_int) -> c_int {
    let text = unsafe {
        match CStr::from_ptr(input_utf8).to_str() {
            Ok(text) => text,
            Err(_) => return ERR_INVALID_ARG,
        }
    };

    f(text)
}

unsafe fn with_utf16_input(input_utf16: *const u16, f: impl FnOnce(&str) -> c_int) -> c_int {
    let text = match utf16_ptr_to_string(input_utf16) {
        Some(text) => text,
        None => return ERR_INVALID_ARG,
    };

    f(&text)
}

unsafe fn write_converted(
    out_buf: *mut c_char,
    buf_size: usize,
    convert: impl FnOnce(&AqKanji2Koe) -> aqkanji2koe::Result<String>,
) -> c_int {
    let inst = match get_instance() {
        Ok(inst) => inst,
        Err(code) => return code,
    };

    let result = match convert(inst) {
        Ok(result) => result,
        Err(_) => return ERR_PROCESSING,
    };

    unsafe { write_cstr(out_buf, buf_size, &result) }
}

unsafe fn write_cstr(buf: *mut c_char, size: usize, s: &str) -> c_int {
    let bytes = s.as_bytes();
    if bytes.len() + 1 > size {
        return ERR_BUFFER_SMALL;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, bytes.len());
        *buf.add(bytes.len()) = 0;
    }
    ERR_OK
}

unsafe fn utf16_ptr_to_string(ptr: *const u16) -> Option<String> {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
        if len > 65536 {
            return None;
        }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16(slice).ok()
}

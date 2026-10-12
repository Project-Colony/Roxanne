//! A native Roxanne plugin that shows the word and line count of the open file
//! in the status bar.
//!
//! Build it with `cargo build --release --example sample_plugin`, then add the
//! absolute path of the library it produces to `plugins.dynamic` in your own
//! `config.toml`. See docs/guide/plugins.md.

use std::borrow::Cow;
use std::ffi::{CStr, CString, c_char};
use std::sync::{Mutex, OnceLock};

/// What the plugin shows in the status bar. Both strings must stay valid until
/// the next call into the plugin.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginStatusV1 {
    pub label: *const c_char,
    pub value: *const c_char,
}

/// Version 1 of the plugin API: the plugin's name and optional callbacks.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginApiV1 {
    pub name: unsafe extern "C" fn() -> *const c_char,
    pub on_text_changed: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_opened: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_saved: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}

static NAME: &CStr = c"sample_plugin";
static LABEL: &CStr = c"Stats";
static VALUE: OnceLock<Mutex<CString>> = OnceLock::new();

fn value() -> &'static Mutex<CString> {
    VALUE.get_or_init(|| Mutex::new(c"0 words, 0 lines".to_owned()))
}

fn update_counts(text: &str) {
    let words = text.split_whitespace().count();
    let lines = text.lines().count().max(1);
    let counts = CString::new(format!("{words} words, {lines} lines")).unwrap_or_default();
    if let Ok(mut value) = value().lock() {
        *value = counts;
    }
}

unsafe extern "C" fn plugin_name() -> *const c_char {
    NAME.as_ptr()
}

unsafe extern "C" fn on_text_changed(text: *const c_char, _filename: *const c_char) {
    let text = if text.is_null() {
        Cow::Borrowed("")
    } else {
        // SAFETY: Roxanne passes a NUL-terminated string that stays valid for
        // the length of the call.
        unsafe { CStr::from_ptr(text) }.to_string_lossy()
    };
    update_counts(&text);
}

unsafe extern "C" fn on_file_opened(text: *const c_char, filename: *const c_char) {
    // SAFETY: same arguments, same contract.
    unsafe { on_text_changed(text, filename) }
}

unsafe extern "C" fn on_file_saved(text: *const c_char, filename: *const c_char) {
    // SAFETY: same arguments, same contract.
    unsafe { on_text_changed(text, filename) }
}

unsafe extern "C" fn status() -> PluginStatusV1 {
    // The pointer stays valid until the next update replaces the string.
    let value = value()
        .lock()
        .map_or(std::ptr::null(), |value| value.as_ptr());
    PluginStatusV1 {
        label: LABEL.as_ptr(),
        value,
    }
}

/// The entry point Roxanne looks up after loading the library.
#[unsafe(no_mangle)]
pub extern "C" fn roxanne_plugin_api_v1() -> PluginApiV1 {
    PluginApiV1 {
        name: plugin_name,
        on_text_changed: Some(on_text_changed),
        on_file_opened: Some(on_file_opened),
        on_file_saved: Some(on_file_saved),
        status: Some(status),
    }
}

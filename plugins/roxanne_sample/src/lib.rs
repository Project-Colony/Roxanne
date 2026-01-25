use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

#[repr(C)]
#[derive(Clone, Copy)]
struct PluginStatusV1 {
    label: *const c_char,
    value: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PluginApiV1 {
    name: unsafe extern "C" fn() -> *const c_char,
    on_text_changed: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    on_file_opened: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    on_file_saved: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}

struct PluginState {
    words: usize,
    lines: usize,
    value: CString,
}

static STATE: OnceLock<Mutex<PluginState>> = OnceLock::new();
static NAME: &[u8; 15] = b"roxanne_sample\0";
static LABEL: &[u8; 6] = b"Stats\0";

fn state() -> &'static Mutex<PluginState> {
    STATE.get_or_init(|| {
        Mutex::new(PluginState {
            words: 0,
            lines: 0,
            value: CString::new("0 mots, 0 lignes").unwrap(),
        })
    })
}

fn update_counts(text: &str) {
    let words = text.split_whitespace().count();
    let lines = text.lines().count().max(1);
    let value = CString::new(format!("{words} mots, {lines} lignes")).unwrap_or_default();
    if let Ok(mut state) = state().lock() {
        state.words = words;
        state.lines = lines;
        state.value = value;
    }
}

unsafe extern "C" fn plugin_name() -> *const c_char {
    NAME.as_ptr() as *const c_char
}

unsafe extern "C" fn on_text_changed(text: *const c_char, _filename: *const c_char) {
    let text = if text.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(text) }.to_string_lossy().as_ref()
    };
    update_counts(text);
}

unsafe extern "C" fn on_file_opened(text: *const c_char, _filename: *const c_char) {
    on_text_changed(text, std::ptr::null());
}

unsafe extern "C" fn on_file_saved(text: *const c_char, _filename: *const c_char) {
    on_text_changed(text, std::ptr::null());
}

unsafe extern "C" fn status() -> PluginStatusV1 {
    let value_ptr = if let Ok(state) = state().lock() {
        state.value.as_ptr()
    } else {
        std::ptr::null()
    };
    PluginStatusV1 {
        label: LABEL.as_ptr() as *const c_char,
        value: value_ptr,
    }
}

#[no_mangle]
pub unsafe extern "C" fn roxanne_plugin_api_v1() -> PluginApiV1 {
    PluginApiV1 {
        name: plugin_name,
        on_text_changed: Some(on_text_changed),
        on_file_opened: Some(on_file_opened),
        on_file_saved: Some(on_file_saved),
        status: Some(status),
    }
}

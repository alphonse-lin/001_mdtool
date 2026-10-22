//! 少量 Win32 封装。

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::null;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE,
};
use windows_sys::Win32::UI::Shell::{ShellExecuteW, SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, SW_SHOWNORMAL};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_YESNOCANCEL,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 弹出模态消息框，返回按下的按钮 ID（IDYES / IDNO / ...）。`owner` 为 0 表示无父窗口。
pub fn message_box(owner: isize, title: &str, text: &str, style: u32) -> i32 {
    unsafe { MessageBoxW(owner as *mut c_void, wide(text).as_ptr(), wide(title).as_ptr(), style) }
}

/// 用系统默认程序打开 URL 或文件。
pub fn shell_open(target: &str) {
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide(target).as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
    }
}

/// 通知资源管理器文件关联已改变。
pub fn refresh_shell() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, null(), null()) }
}

/// 系统「应用」是否使用浅色模式（读取失败时按浅色处理）。
pub fn is_light_theme() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .map(|v| v != 0)
        .unwrap_or(true)
}

pub fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

// ---------- 后台常驻进程的进程间信号 ----------

const BACKGROUND_MUTEX: &str = r"Local\mdtool-background";
const BACKGROUND_QUIT: &str = r"Local\mdtool-background-quit";

/// 占用「后台进程」名额；已有后台进程在运行时返回 false。互斥量句柄随进程退出释放。
pub fn claim_background() -> bool {
    unsafe {
        let handle = CreateMutexW(null(), 0, wide(BACKGROUND_MUTEX).as_ptr());
        !handle.is_null() && GetLastError() != ERROR_ALREADY_EXISTS
    }
}

/// 阻塞直到收到退出信号。
pub fn wait_background_quit() {
    unsafe {
        let event = CreateEventW(null(), 1, 0, wide(BACKGROUND_QUIT).as_ptr());
        WaitForSingleObject(event, INFINITE);
    }
}

/// 通知后台进程退出；没有后台进程时返回 false。
pub fn stop_background() -> bool {
    unsafe {
        let event = OpenEventW(EVENT_MODIFY_STATE, 0, wide(BACKGROUND_QUIT).as_ptr());
        if event.is_null() {
            return false;
        }
        SetEvent(event);
        CloseHandle(event);
        true
    }
}

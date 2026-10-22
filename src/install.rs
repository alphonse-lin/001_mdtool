//! 安装到 %LOCALAPPDATA%\Programs\mdtool 并注册 .md 文件关联（只写当前用户注册表）。

use std::io;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_ALL_ACCESS};
use winreg::{RegKey, RegValue};

use crate::i18n::tr;
use crate::win;

const PROG_ID: &str = "mdtool.md";
const EXTENSIONS: [&str; 2] = [".md", ".markdown"];
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

const README_ZH: &str = "# mdtool 已安装

现在请在弹出的「打开方式」窗口中选择 **mdtool**，并点「始终」。之后双击任何 `.md` 文件都会用 mdtool 打开。

如果没有弹出窗口：右键任意 `.md` 文件 → 打开方式 → 选择其他应用 → mdtool → 始终。

## 快捷键

| 快捷键 | 作用 |
|---|---|
| `Ctrl+E` | 切换阅读 / 编辑 |
| `Ctrl+S` | 保存 |
| `Ctrl+F` | 查找 |
| `Ctrl+滚轮` | 缩放 |

## 卸载

运行 `mdtool.exe --uninstall`，然后删除本目录。

## 后台进程

为了让双击打开更快，mdtool 会在开机时启动一个隐藏的后台进程，让 WebView2 一直处于就绪状态
（有它时打开约 0.25 秒，没有时约 0.6 秒）。不需要的话，在「任务管理器 → 启动应用」中禁用 mdtool 即可。
";

const README_EN: &str = "# mdtool is installed

In the **Open with** dialog, pick **mdtool** and click **Always**. From then on, double-clicking any `.md` file opens it in mdtool.

No dialog? Right-click any `.md` file → Open with → Choose another app → mdtool → Always.

## Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+E` | Toggle reading / editing |
| `Ctrl+S` | Save |
| `Ctrl+F` | Find |
| `Ctrl+Mouse wheel` | Zoom |

## Uninstall

Run `mdtool.exe --uninstall`, then delete this folder.

## Background helper

To open files faster, mdtool starts a hidden background helper at login that keeps WebView2 ready
(about 0.25 s per file with it, about 0.6 s without). If you don't want it, disable mdtool in Task Manager → Startup apps.
";

fn install_dir() -> PathBuf {
    win::local_app_data().join("Programs").join("mdtool")
}

/// 当前运行的是否就是已安装的那份 exe。
pub fn is_installed_copy() -> bool {
    let current = std::env::current_exe().and_then(std::fs::canonicalize);
    let installed = std::fs::canonicalize(install_dir().join("mdtool.exe"));
    matches!((current, installed), (Ok(a), Ok(b)) if a == b)
}

fn classes() -> io::Result<RegKey> {
    Ok(RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"Software\Classes")?.0)
}

pub fn install() -> io::Result<String> {
    let dir = install_dir();
    std::fs::create_dir_all(&dir)?;
    let exe = dir.join("mdtool.exe");
    let current = std::env::current_exe()?;
    if std::fs::canonicalize(&current).ok() != std::fs::canonicalize(&exe).ok() {
        // 旧版本的后台进程会占用 exe，先让它退出
        if win::stop_background() {
            std::thread::sleep(Duration::from_millis(500));
        }
        std::fs::copy(&current, &exe)?;
    }
    let readme = dir.join("README.md");
    std::fs::write(&readme, tr(README_ZH, README_EN))?;

    let exe = exe.display().to_string();
    let command = format!("\"{exe}\" \"%1\"");
    let classes = classes()?;

    let (prog, _) = classes.create_subkey(PROG_ID)?;
    prog.set_value("", &tr("Markdown 文档", "Markdown Document"))?;
    prog.create_subkey("DefaultIcon")?.0.set_value("", &format!("\"{exe}\",0"))?;
    prog.create_subkey(r"shell\open\command")?.0.set_value("", &command)?;

    let (app, _) = classes.create_subkey(r"Applications\mdtool.exe")?;
    app.set_value("FriendlyAppName", &"mdtool")?;
    app.create_subkey(r"shell\open\command")?.0.set_value("", &command)?;
    let (types, _) = app.create_subkey("SupportedTypes")?;
    for ext in EXTENSIONS {
        types.set_value(ext, &"")?;
        let (owp, _) = classes.create_subkey(format!(r"{ext}\OpenWithProgids"))?;
        owp.set_raw_value(PROG_ID, &RegValue { vtype: RegType::REG_NONE, bytes: Vec::new().into() })?;
    }
    win::refresh_shell();

    // 开机启动后台进程，并立即启动一个
    RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(RUN_KEY)?
        .0
        .set_value("mdtool", &format!("\"{exe}\" --background"))?;
    Command::new(&exe).arg("--background").spawn()?;

    // Windows 11 不允许程序直接设置默认应用，弹出系统「打开方式」让用户选一次
    Command::new("rundll32.exe")
        .raw_arg(format!("shell32.dll,OpenAs_RunDLL {}", readme.display()))
        .spawn()?;

    Ok(format!(
        "{}\n{exe}\n\n{}",
        tr("已安装到：", "Installed to:"),
        tr(
            "接下来在「打开方式」窗口中选择 mdtool，并点「始终」。",
            "Now pick mdtool in the \"Open with\" dialog and click \"Always\".",
        ),
    ))
}

pub fn uninstall() -> io::Result<String> {
    if let Ok(run) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_ALL_ACCESS) {
        let _ = run.delete_value("mdtool");
    }
    win::stop_background();
    let classes = classes()?;
    let _ = classes.delete_subkey_all(PROG_ID);
    let _ = classes.delete_subkey_all(r"Applications\mdtool.exe");
    for ext in EXTENSIONS {
        if let Ok(owp) = classes.open_subkey_with_flags(format!(r"{ext}\OpenWithProgids"), KEY_ALL_ACCESS) {
            let _ = owp.delete_value(PROG_ID);
        }
    }
    win::refresh_shell();
    Ok(format!(
        "{}\n{}",
        tr(
            "已取消 .md 文件关联和开机后台进程。\n如需彻底删除，请手动删除目录：",
            "Removed the .md file association and the startup helper.\nTo remove mdtool completely, delete this folder:",
        ),
        install_dir().display()
    ))
}

#![windows_subsystem = "windows"]

mod doc;
mod i18n;
mod install;
mod protocol;
mod watch;
mod win;

use std::io::{self, Write as _};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{json, Value};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::windows::{IconExtWindows, WindowExtWindows};
use tao::window::{Icon, Theme, WindowBuilder};
use wry::{NewWindowResponse, ScrollBarStyle, WebContext, WebViewBuilder, WebViewBuilderExtWindows};

use doc::{Decoded, Encoding};
use i18n::tr;

enum UserEvent {
    Ipc(String),
    FileChanged(Decoded),
}

/// 当前打开并可保存的文档。
struct OpenDoc {
    path: PathBuf,
    encoding: Encoding,
}

/// 点击这些类型的本地文件链接时只在资源管理器中定位，不直接运行。
const RUNNABLE: &[&str] = &[
    "exe", "com", "bat", "cmd", "msi", "ps1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "scr",
    "lnk", "hta", "cpl", "reg", "pif", "jar",
];

fn main() {
    let started = Instant::now();
    // 没有控制台，panic 信息写到 %LOCALAPPDATA%\mdtool\crash.log
    std::panic::set_hook(Box::new(|info| {
        let dir = win::local_app_data().join("mdtool");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("crash.log")) {
            let _ = writeln!(f, "{info}\n{}", std::backtrace::Backtrace::force_capture());
        }
    }));
    let arg = std::env::args_os().nth(1);
    match arg.as_ref().and_then(|a| a.to_str()) {
        Some("--install") => return report(install::install()),
        Some("--uninstall") => return report(install::uninstall()),
        Some("--background") => return background(),
        None if !install::is_installed_copy() => {
            // 直接双击下载的 exe：询问是否安装（已安装过则相当于升级）
            let ask = tr(
                "要安装 mdtool 并用它打开 .md 文件吗？\n\n只安装到当前用户，不需要管理员权限。\n选「否」则只显示使用说明。",
                "Install mdtool and use it to open .md files?\n\nIt installs for the current user only, no admin rights needed.\nChoose No to just see the usage page.",
            );
            if win::message_box(0, "mdtool", ask, win::MB_YESNO | win::MB_ICONQUESTION) == win::IDYES {
                return report(install::install());
            }
        }
        _ => {}
    }
    let path = arg.map(PathBuf::from).map(|p| std::path::absolute(&p).unwrap_or(p));
    run(path, started);
}

fn report(result: io::Result<String>) {
    match result {
        Ok(msg) => win::message_box(0, "mdtool", &msg, win::MB_ICONINFORMATION),
        Err(e) => win::message_box(0, "mdtool", &format!("{}{e}", tr("操作失败：", "Failed: ")), win::MB_ICONERROR),
    };
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn window_title(name: Option<&str>, dirty: bool) -> String {
    match name {
        Some(name) => format!("{}{name} - mdtool", if dirty { "● " } else { "" }),
        None => "mdtool".into(),
    }
}

fn web_context() -> WebContext {
    WebContext::new(Some(win::local_app_data().join("mdtool").join("WebView2")))
}

/// 所有 WebView 共用的构建器。WebView2 只有在用户数据目录和环境选项（如滚动条样式）完全一致时
/// 才会在进程间共享浏览器进程，所以后台进程和查看窗口必须都从这里创建。
fn webview_builder(context: &mut WebContext) -> WebViewBuilder<'_> {
    WebViewBuilder::new_with_web_context(context).with_scroll_bar_style(ScrollBarStyle::FluentOverlay)
}

/// 后台常驻：在隐藏窗口里保持一个空白 WebView，让 WebView2 浏览器进程一直运行。
/// 之后打开文件的新进程直接复用它（实测首次渲染 ~580ms → ~230ms）。
fn background() {
    if !win::claim_background() {
        return;
    }
    std::thread::spawn(|| {
        win::wait_background_quit();
        std::process::exit(0);
    });
    let event_loop = EventLoopBuilder::<()>::new().build();
    let window = WindowBuilder::new().with_visible(false).build(&event_loop).expect("failed to create window");
    let mut context = web_context();
    let Ok(webview) = webview_builder(&mut context).with_html("").build(&window) else { return };
    event_loop.run(move |_, _, flow| {
        *flow = ControlFlow::Wait;
        let _ = (&webview, &context, &window);
    });
}

fn run(path: Option<PathBuf>, started: Instant) {
    // ---------- 读取文档，组装注入给前端的初始数据 ----------
    let last_mtime = Arc::new(Mutex::new(None));
    let name = path.as_deref().map(file_name);
    let mut init = json!({
        "lang": i18n::code(),
        "debug": std::env::var_os("MDTOOL_TIMING").is_some(),
    });
    let mut doc: Option<OpenDoc> = None;
    if let Some(path) = &path {
        let dir = path.parent().map(|d| d.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        init["path"] = path.to_string_lossy().into();
        init["name"] = name.clone().into();
        init["dir"] = dir.trim_end_matches('/').into();
        match doc::load(path) {
            Ok(d) => {
                *last_mtime.lock().unwrap() = doc::mtime(path);
                let encoding = if d.lossy { Encoding::Utf8 } else { d.encoding };
                init["lossy"] = d.lossy.into();
                init["text"] = d.text.into();
                doc = Some(OpenDoc { path: path.clone(), encoding });
            }
            Err(e) => init["error"] = e.to_string().into(),
        }
    }

    // ---------- 窗口 ----------
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let light = win::is_light_theme();
    let bg = if light { (255, 255, 255, 255) } else { (13, 17, 23, 255) };
    let window = WindowBuilder::new()
        .with_title(window_title(name.as_deref(), false))
        .with_inner_size(LogicalSize::new(1000.0, 800.0))
        .with_theme(Some(if light { Theme::Light } else { Theme::Dark }))
        .with_background_color(bg)
        .with_window_icon(Icon::from_resource(1, None).ok())
        .build(&event_loop)
        .expect("failed to create window");
    let hwnd = window.hwnd();
    let t_window = started.elapsed().as_millis();

    // ---------- WebView ----------
    let mut web_context = web_context();
    let proxy = event_loop.create_proxy();
    let webview = webview_builder(&mut web_context)
        .with_background_color(bg)
        .with_initialization_script(format!("window.__MDTOOL__ = {init};"))
        .with_custom_protocol("mdtool".into(), |_id, request| protocol::handle(request))
        .with_ipc_handler(move |request| {
            let _ = proxy.send_event(UserEvent::Ipc(request.into_body()));
        })
        .with_navigation_handler(|url| {
            let internal = url.starts_with("http://mdtool.localhost/") || url.starts_with("mdtool://");
            if !internal {
                open_url(&url);
            }
            internal
        })
        .with_new_window_req_handler(|url, _| {
            open_url(&url);
            NewWindowResponse::Deny
        })
        .with_url("mdtool://localhost/app/index.html")
        .with_devtools(cfg!(debug_assertions))
        .build(&window);
    let webview = match webview {
        Ok(w) => w,
        Err(e) => {
            win::message_box(hwnd, "mdtool", &format!(
                "{}{e}\n{}",
                tr("无法创建 WebView2：", "Could not create WebView2: "),
                tr("请确认已安装 Microsoft Edge WebView2 运行时。", "Please make sure the Microsoft Edge WebView2 Runtime is installed."),
            ), win::MB_ICONERROR);
            return;
        }
    };

    let t_webview = started.elapsed().as_millis();
    if let Some(d) = &doc {
        let proxy = event_loop.create_proxy();
        watch::spawn(d.path.clone(), last_mtime.clone(), move |decoded| {
            let _ = proxy.send_event(UserEvent::FileChanged(decoded));
        });
    }

    // ---------- 事件循环 ----------
    let mut dirty = false;
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        let _ = &web_context;
        let eval = |js: String| {
            let _ = webview.evaluate_script(&js);
        };
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if !dirty {
                    *flow = ControlFlow::Exit;
                    return;
                }
                let msg = tr("是否保存对“{}”的修改？", "Save changes to \"{}\"?").replace("{}", name.as_deref().unwrap_or(""));
                match win::message_box(hwnd, "mdtool", &msg, win::MB_YESNOCANCEL | win::MB_ICONWARNING) {
                    win::IDYES => eval("mdtool.requestSaveAndClose()".into()),
                    win::IDNO => *flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            Event::UserEvent(UserEvent::Ipc(body)) => {
                let Ok(msg) = serde_json::from_str::<Value>(&body) else { return };
                match msg["cmd"].as_str() {
                    Some("save") => {
                        let text = msg["text"].as_str().unwrap_or_default();
                        let close = msg["close"].as_bool().unwrap_or(false);
                        match save(doc.as_mut(), &last_mtime, text) {
                            Ok(()) if close => *flow = ControlFlow::Exit,
                            Ok(()) => eval("mdtool.onSaved(true, '')".into()),
                            Err(e) => eval(format!("mdtool.onSaved(false, {})", Value::from(e.to_string()))),
                        }
                    }
                    Some("dirty") => {
                        dirty = msg["value"].as_bool().unwrap_or(false);
                        window.set_title(&window_title(name.as_deref(), dirty));
                    }
                    Some("open") => open_link(hwnd, &msg),
                    Some("log") => log_timing(started, None, msg["msg"].as_str().unwrap_or_default()),
                    Some("rendered") => log_timing(started, path.as_deref(), &format!("window={t_window} webview={t_webview} js={}", msg["perf"])),
                    _ => {}
                }
            }
            Event::UserEvent(UserEvent::FileChanged(decoded)) => {
                if let Some(d) = doc.as_mut() {
                    d.encoding = if decoded.lossy { Encoding::Utf8 } else { decoded.encoding };
                }
                eval(format!("mdtool.onExternalChange({})", Value::from(decoded.text)));
            }
            _ => {}
        }
    });
}

/// 持锁写入并更新已知修改时间，这样监听线程不会把这次保存当成外部修改。
fn save(doc: Option<&mut OpenDoc>, last_mtime: &Mutex<Option<std::time::SystemTime>>, text: &str) -> io::Result<()> {
    let doc = doc.ok_or_else(|| io::Error::other(tr("没有打开的文件", "No file is open")))?;
    let mut last = last_mtime.lock().unwrap();
    doc.encoding = doc::save(&doc.path, text, doc.encoding)?;
    *last = doc::mtime(&doc.path);
    Ok(())
}

/// 只把网页、邮件链接交给系统打开。
fn open_url(url: &str) {
    let lower = url.to_ascii_lowercase();
    if ["http://", "https://", "mailto:"].iter().any(|p| lower.starts_with(p)) {
        win::shell_open(url);
    }
}

fn open_link(hwnd: isize, msg: &Value) {
    if let Some(url) = msg["url"].as_str() {
        return open_url(url);
    }
    let Some(target) = msg["path"].as_str() else { return };
    let path = Path::new(target);
    if !path.exists() {
        win::message_box(hwnd, "mdtool", &format!("{}\n{}", tr("文件不存在：", "File not found:"), path.display()), win::MB_ICONWARNING);
        return;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if ext == "md" || ext == "markdown" {
        if let Ok(exe) = std::env::current_exe() {
            let _ = Command::new(exe).arg(path).spawn();
        }
    } else if RUNNABLE.contains(&ext.as_str()) {
        let _ = Command::new("explorer.exe").raw_arg(format!("/select,\"{}\"", path.display())).spawn();
    } else {
        win::shell_open(target);
    }
}

/// MDTOOL_TIMING=1 时记录「进程启动 → 首次渲染完成」耗时。
fn log_timing(started: Instant, path: Option<&Path>, detail: &str) {
    if std::env::var_os("MDTOOL_TIMING").is_none() {
        return;
    }
    let dir = win::local_app_data().join("mdtool");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("timing.log")) {
        let _ = writeln!(f, "{}\t{detail}\t{}", started.elapsed().as_millis(), path.map(|p| p.display().to_string()).unwrap_or_default());
    }
}

# mdtool 设计文档

日期：2026-10-22

## 目标

在 Windows 11 上双击 `.md` 文件，**尽快**弹出一个窗口显示渲染好的 Markdown；需要时可切换到编辑模式修改并保存。

成功标准：
- 从启动进程到内容显示 ≤ 0.5 秒（本机实测，WebView2 冷启动除外的常规情况）。
- 单个 exe，离线可用，不依赖额外运行时（WebView2 为 Win11 自带）。

## 技术选型

- Rust：`tao`（窗口）+ `wry`（WebView2）。
- 前端资源用 `include_dir` 嵌入 exe，通过自定义协议 `mdtool://` 提供（Windows 上实际为 `http://mdtool.localhost/`）。
- 第三方前端库（本地化，不走 CDN）：markdown-it、markdown-it-task-lists、markdown-it-footnote、markdown-it-texmath + KaTeX、highlight.js、CodeMirror 5（markdown/gfm 模式）。
- 其他 crate：`serde_json`（IPC）、`encoding_rs`（GBK）、`winreg`（文件关联）、`windows-sys`（MessageBox、ShellExecute、SHChangeNotify）。

## 结构

```
Cargo.toml
build.rs              # 嵌入图标与 manifest
src/
  main.rs             # 参数解析、窗口与事件循环、IPC 分发
  doc.rs              # 读取/解码/编码/保存文档（含编码回退），纯逻辑可测
  protocol.rs         # 自定义协议：/app/* 嵌入资源，/doc/* 文档目录下的本地文件
  watch.rs            # 轮询文件 mtime，外部修改时通知主线程
  install.rs          # --install / --uninstall 注册表操作
  win.rs              # MessageBox、打开 URL 等 Win32 小封装
assets/
  index.html  app.js  style.css
  vendor/...
tests/fixtures/       # 编码测试样例
sample.md             # 覆盖全部语法的手测文档
```

## 启动流程（性能关键路径）

1. `main` 解析参数，读取文件并解码（UTF-8 → 失败回退 GBK）。
2. 根据注册表 `AppsUseLightTheme` 决定窗口背景色，创建窗口并**立即显示**（背景色与主题一致，避免白屏闪烁）。
3. 创建 WebView，WebView2 用户数据目录固定为 `%LOCALAPPDATA%\mdtool\WebView2`。
4. 文档内容通过 initialization script 注入为 `window.__MDTOOL__ = {path, name, text}`，省去一次 IPC 往返。
5. `index.html` 只同步加载 markdown-it（及任务列表、脚注插件）与 CSS；渲染后：
   - 文档含 `$` → 动态加载 KaTeX + texmath 后重新渲染；
   - 文档含代码块 → 动态加载 highlight.js 后对代码块上色；
   - CodeMirror 仅在首次按 Ctrl+E 时加载。
6. 环境变量 `MDTOOL_TIMING=1` 时，前端渲染完成后通过 IPC 上报，Rust 把「进程启动 → 首次渲染完成」的毫秒数追加写入 `%LOCALAPPDATA%\mdtool\timing.log`，用于实测。

## 功能

- **阅读模式（默认）**：GitHub 风格排版，跟随系统深/浅色（CSS `prefers-color-scheme`）。窗口标题为文件名。
- **编辑模式（Ctrl+E 切换）**：左 CodeMirror 源码、右实时预览（防抖 150ms）。预览元素带 `data-line`（来自 markdown-it token.map），编辑器滚动时预览按行号同步。再按 Ctrl+E 回到阅读模式（显示最新内容）。
- **保存（Ctrl+S）**：JS 发送 `{cmd:"save", text}`；Rust 按原编码写回（UTF-8 保留原 BOM 状态；GBK 写回 GBK），成功后更新已知 mtime，避免触发自身的"外部修改"。有未保存修改时标题前加 `● `。
- **关闭确认**：JS 在 dirty 状态变化时通知 Rust。关闭窗口时若 dirty，弹 MessageBox「是否保存修改？」是/否/取消：是 → 让前端带内容保存后退出；否 → 直接退出；取消 → 不关闭。
- **外部修改**：后台线程每 700ms 检查 mtime。变化时重新读取并推给前端：未 dirty → 静默刷新（保留滚动位置）；dirty → 顶部横幅「文件已在外部修改」[重新加载] [保留我的]。
- **链接与图片**：
  - 相对路径的 `img src` / `a href` 在前端改写为 `/doc/<相对路径>`，由 Rust 从文档所在目录读取返回。
  - `http(s)://`、`mailto:` 链接 → IPC 交给系统默认程序打开。
  - 指向其他 `.md` 的相对链接 → 用新进程打开该文件。
  - `#锚点` → 页内跳转（标题自动生成 id）。
- **无参数启动**：显示一页使用说明（用法与 `--install`）。

## 安装（文件关联）

- `mdtool.exe --install`：把自身复制到 `%LOCALAPPDATA%\Programs\mdtool\mdtool.exe`，在 `HKCU\Software\Classes` 下注册：
  - ProgID `mdtool.md`（`shell\open\command` = `"<exe>" "%1"`，`DefaultIcon`）；
  - `.md`、`.markdown` 的 `OpenWithProgids` 加入 `mdtool.md`；
  - `Applications\mdtool.exe`（`FriendlyAppName`、`SupportedTypes`）。
  - 调用 `SHChangeNotify` 刷新资源管理器。
- Windows 11 不允许程序直接改默认程序（UserChoice 有哈希保护）。因此安装完成后弹出系统「打开方式」对话框（`rundll32 shell32.dll,OpenAs_RunDLL <安装目录>\README.md`；README.md 由安装时写出，内容为使用说明），用户选 mdtool 并勾选「始终」——只需做一次。
- `mdtool.exe --uninstall`：删除上述注册表项。

## 错误处理

- 文件不存在/读取失败：窗口中显示错误信息与路径。
- 解码：UTF-8（含 BOM）→ GBK；GBK 也有无效字节时按 UTF-8 有损解码并在顶部提示「编码无法识别，保存可能损坏内容」，此时保存按 UTF-8 写。
- 保存失败（只读、占用）：前端弹提示，内容保留在编辑器中，dirty 不清除。
- `/doc/*` 请求的文件不存在 → 404，图片显示为破图，不影响其他内容。

## 测试

- Rust 单元测试：`doc.rs` 的解码/编码往返（UTF-8、UTF-8 BOM、GBK、无效字节）、保存后 mtime 更新；`protocol.rs` 的路径解析（百分号解码、相对路径、MIME）。
- 手测：`sample.md` 覆盖标题、列表、表格、链接、本地图片、代码高亮、行内/块级公式、任务列表、脚注；检查阅读/编辑切换、保存、关闭确认、外部修改。
- 性能：`MDTOOL_TIMING=1` 下多次启动 `sample.md`，报告首次渲染耗时。

## 不做（YAGNI）

标签页/多文件、文件树、所见即所得编辑、导出 PDF/HTML、Mermaid、后台常驻进程（仅当实测远超 0.5s 时再考虑）。

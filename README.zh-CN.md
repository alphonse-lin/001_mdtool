<p align="center">
  <img src="res/icon.png" width="96" alt="mdtool logo">
</p>

<h1 align="center">mdtool</h1>

<p align="center">
  <b>双击 <code>.md</code> 文件，0.25 秒看到渲染结果。</b><br>
  Windows 上小巧、快速的 Markdown 查看 / 编辑器。
</p>

<p align="center">
  <a href="../../releases/latest"><img alt="下载" src="https://img.shields.io/badge/下载-mdtool.exe-2563eb?style=for-the-badge&logo=windows"></a>
</p>

<p align="center">
  <a href="README.md">English</a> · 简体中文
</p>

<p align="center">
  <img src="docs/images/reading.png" width="820" alt="阅读模式">
</p>

---

## 为什么做 mdtool？

在 Windows 上「只是想看一眼这个 Markdown 文件」一直很麻烦：记事本显示的是源码，VS Code 启动要好几秒，Typora 要付费，浏览器插件配置又繁琐。

mdtool 只做好一件事：**让 `.md` 文件像图片一样打开**——双击即渲染，独立窗口。需要改个错别字时，按 <kbd>Ctrl</kbd>+<kbd>E</kbd>。

## 功能

- ⚡ **快**：从双击到渲染完成约 **0.25 秒**（见[性能](#性能)）
- 📦 **小**：单个约 2.4 MB 的 `.exe`，完全离线，无需安装运行时
- 🎨 **GitHub 风格排版**，跟随系统深色 / 浅色主题
- ✍️ **内置编辑器**：<kbd>Ctrl</kbd>+<kbd>E</kbd> 打开左右分栏编辑，实时预览、滚动同步
- 🧮 **完整的 Markdown 支持**：表格、任务列表、脚注、代码高亮（36 种常用语言）、KaTeX 数学公式
- 🖼️ **本地图片直接显示**：`../images/diagram.png` 这类相对路径都能正确解析
- 🔄 **自动刷新**：文件被其他程序修改后，窗口自动更新
- 🛡️ **安全**：不会覆盖你未保存的修改；保存时保持原编码（UTF-8 / UTF-8 BOM / GBK）；阻止文档中嵌入的脚本

<p align="center">
  <img src="docs/images/editing.png" width="820" alt="编辑模式">
  <br><sub>编辑模式：左侧源码，右侧实时预览。</sub>
</p>

## 安装

1. 从 [最新 Release](../../releases/latest) 下载 **`mdtool.exe`**。
2. 在下载目录打开终端，运行：

   ```powershell
   .\mdtool.exe --install
   ```

3. 系统会弹出「打开方式」窗口，选择 **mdtool**，点「**始终**」。

完成。之后所有 `.md` / `.markdown` 文件都会用 mdtool 打开。

> [!NOTE]
> **提示「Windows 已保护你的电脑」？** 程序没有代码签名，SmartScreen 第一次可能会拦截。点「**更多信息 → 仍要运行**」即可。也可以[从源码自己编译](#从源码构建)。

<details>
<summary><b><code>--install</code> 具体做了什么？</b></summary>

<br>

全部在当前用户范围内完成，**不需要管理员权限**：

- 把 `mdtool.exe` 复制到 `%LOCALAPPDATA%\Programs\mdtool\`
- 在 `HKEY_CURRENT_USER\Software\Classes` 下把它注册为 `.md` / `.markdown` 的「打开方式」选项
- 在 `HKCU\...\CurrentVersion\Run` 下添加[后台进程](#性能)的开机启动项
- 弹出系统「打开方式」窗口（Windows 11 不允许程序自行设置默认应用）

</details>

### 卸载

```powershell
& "$env:LOCALAPPDATA\Programs\mdtool\mdtool.exe" --uninstall
```

会删除文件关联和开机启动项，之后手动删除 `%LOCALAPPDATA%\Programs\mdtool` 目录即可。

## 使用

| 操作 | 效果 |
|---|---|
| 双击 `.md` 文件 | 以阅读模式打开渲染结果 |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> | 切换阅读 / 分栏编辑 |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | 保存（保持原编码） |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> | 页内查找 |
| <kbd>Ctrl</kbd>+<kbd>鼠标滚轮</kbd> | 缩放 |
| 点击链接 | 网页链接用浏览器打开；指向其他 `.md` 的链接在新的 mdtool 窗口中打开 |
| 有未保存修改时关闭 | 询问是否保存 |

也可以在命令行中使用：`mdtool.exe path\to\file.md`。

## 性能

在作者的 Windows 11 电脑上，从进程启动到首次渲染完成：

| 场景 | 耗时 |
|---|---|
| 后台进程运行中（安装后默认） | **约 0.25 秒** |
| 冷启动（无后台进程） | 约 0.6 秒 |

冷启动的大部分时间花在拉起 WebView2 浏览器引擎上。为此 `--install` 会注册一个开机启动的隐藏**后台进程**（`mdtool.exe --background`）让 WebView2 保持就绪，之后打开的每个文件都直接复用它。

代价是后台常驻约 **160 MB 内存**。不需要的话，在「任务管理器 → 启动应用」中禁用 **mdtool** 即可，功能完全不受影响，只是打开变为约 0.6 秒。

## 系统要求

- **Windows 11**（x64）
- Microsoft Edge **WebView2 运行时**（Windows 11 自带）

装有 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/) 的 Windows 10 理论上也可以运行，但未经测试。

## 隐私与安全

- mdtool **不会联网上报任何信息**：没有遥测，没有更新检查。唯一的网络请求来自文档中引用的远程图片。
- Markdown 文件可以包含 HTML。mdtool 会渲染它，但严格的内容安全策略（CSP）会阻止其中的脚本、事件处理器、iframe 和表单。恶意的 `.md` 文件无法执行代码或改动你的文件。
- 点击指向可执行文件（`.exe`、`.bat`、`.ps1` 等）的链接只会在资源管理器中定位该文件，不会运行它。

## 工作原理

```
mdtool.exe  ──读取并解码──▶  your.md
    │
    ├─ tao        原生窗口
    ├─ wry        WebView2（Windows 内置的 Edge 引擎）
    │              └─ markdown-it · KaTeX · highlight.js · CodeMirror
    │                 全部嵌入 exe，按需懒加载
    └─ IPC        保存 · 打开链接 · 关闭确认 · 文件监听
```

- **Rust** 负责窗口、文件读写（含编码识别）、文件监听和安装。
- 文档内容在页面加载前就注入，首次渲染无需额外往返。
- KaTeX、highlight.js 和编辑器只在文档需要时才加载。

## 从源码构建

需要 Windows 上的 [Rust](https://rustup.rs/)（stable，MSVC 工具链）。克隆本仓库后：

```powershell
cargo build --release          # → target\release\mdtool.exe
cargo test                     # 单元测试
```

前端第三方库已放在 `assets/vendor/`。如需重新下载，运行 `sh scripts/fetch-vendor.sh`（例如在 Git Bash 中）。

### 排查问题

启动前设置环境变量 `MDTOOL_TIMING=1`，启动耗时和前端错误会写入 `%LOCALAPPDATA%\mdtool\timing.log`；程序崩溃信息写入 `%LOCALAPPDATA%\mdtool\crash.log`。

## 路线图

- [ ] 英文界面（目前按钮和对话框为中文）
- [ ] 代码签名的发布版本
- [ ] Mermaid 图表
- [ ] 导出 HTML / PDF

欢迎提 Issue 和 Pull Request！

## 致谢

mdtool 基于以下优秀的开源项目：

| 项目 | 用途 | 许可证 |
|---|---|---|
| [tao](https://github.com/tauri-apps/tao) / [wry](https://github.com/tauri-apps/wry) | 窗口与 WebView2 | MIT / Apache-2.0 |
| [markdown-it](https://github.com/markdown-it/markdown-it) | Markdown 解析 | MIT |
| [markdown-it-footnote](https://github.com/markdown-it/markdown-it-footnote) | 脚注 | MIT |
| [markdown-it-task-lists](https://github.com/revin/markdown-it-task-lists) | 任务列表 | ISC |
| [markdown-it-texmath](https://github.com/goessner/markdown-it-texmath) | 公式语法 | MIT |
| [KaTeX](https://github.com/KaTeX/KaTeX) | 公式渲染 | MIT |
| [highlight.js](https://github.com/highlightjs/highlight.js) | 代码高亮 | BSD-3-Clause |
| [CodeMirror 5](https://github.com/codemirror/codemirror5) | 编辑器 | MIT |
| [encoding_rs](https://github.com/hsivonen/encoding_rs) | GBK 支持 | MIT / Apache-2.0 |

## 许可证

[MIT](LICENSE)

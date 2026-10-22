# mdtool

Windows 上的轻量 Markdown 查看/编辑器：双击 `.md` 文件，约 0.25 秒弹出渲染好的页面。

- 阅读模式（默认）：GitHub 风格排版，跟随系统深色/浅色
- `Ctrl+E` 切换编辑模式：左侧源码、右侧实时预览，滚动同步
- `Ctrl+S` 保存（保持原文件编码：UTF-8 / UTF-8 BOM / GBK）
- 支持代码高亮、数学公式（`$...$`、`$$...$$`）、任务列表、脚注、相对路径图片
- 文件被其他程序修改时自动刷新；有未保存修改时会先询问
- 单个 exe（约 2.5MB），离线可用，依赖 Win11 自带的 WebView2

## 构建

```sh
cargo build --release            # 产物：target/release/mdtool.exe
cargo test                       # 单元测试
sh scripts/fetch-vendor.sh       # （可选）重新下载前端第三方库
```

## 安装 / 卸载

```sh
target/release/mdtool.exe --install     # 复制到 %LOCALAPPDATA%\Programs\mdtool 并注册 .md 关联
mdtool.exe --uninstall                  # 取消关联和开机后台进程
```

Windows 11 不允许程序自行设置默认应用，安装后会弹出「打开方式」窗口，选择 mdtool 并点「始终」即可（只需一次）。

### 后台进程

WebView2 冷启动需要约 0.45 秒拉起浏览器进程。安装时会注册一个开机启动的隐藏后台进程（`mdtool.exe --background`）
保持 WebView2 就绪，之后每次打开只需约 0.25 秒；代价是常驻约 160MB 内存。
不需要的话在「任务管理器 → 启动应用」中禁用 mdtool。

## 调试

设置环境变量 `MDTOOL_TIMING=1` 后启动，耗时和前端错误会写入 `%LOCALAPPDATA%\mdtool\timing.log`；
程序崩溃信息写入 `%LOCALAPPDATA%\mdtool\crash.log`。

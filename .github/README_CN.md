# OBS-MPV 同步控制工具 (om)

> [!NOTE]
> 这是一个基于 Rust 开发的高效实用工具，旨在让你能够同步控制 OBS Studio 的录制和 MPV 的播放状态。它专为 **配音录制** 和 **内容创作** 场景优化，支持逐条手动播放以及同步的歌词/字幕通知显示。

# 新功能

* **同步控制**：通过全局快捷键同时切换 OBS 和 MPV 的暂停/恢复状态。
* **手动步进播放**：不再自动播放下一条，仅在触发信号时加载新文件，且在每段素材播放结束时自动停留在末尾。
* **GPT-SoVITS 集成**：自动解析 `slicer_opt.list` 文件，并将歌词/文本以系统通知的形式显示（Linux 使用 `notify-send`，Windows 使用 Toast 通知）。
* **智能路径映射**：即使 list 文件包含 Windows 风格的相对路径或文件被移动，程序也能自动解析并定位音频文件。
* **安全退出**：退出程序时自动停止 OBS 录制并清理 MPV 进程。

# 软硬件要求

* Linux (COSMIC/Sway/X11) 或 Windows 10/11
* [MPV 播放器](https://mpv.io/)
* [OBS Studio](https://obsproject.com/) 且开启 WebSocket 服务（可选，若仅使用 mpv 播放则不需要）
* [obs-cmd](https://github.com/grigio/obs-cmd) (请确保其已添加到系统环境变量 PATH 中)（可选，若仅使用 mpv 播放则不需要）

# 使用方法

### 1. 全局快捷键设置

你需要在系统中设置两个快捷键，用于在系统临时目录中创建“触发文件”。

#### **Windows 平台 (AutoHotkey v2):**

[https://www.autohotkey.com/](https://www.autohotkey.com/)

```ahk
#Requires AutoHotkey v2.0

; Alt + D: 切换 暂停/恢复
!d:: {
    FileAppend("", EnvGet("TEMP") . "\obs_mpv_toggle_pause")
}

; Alt + S: 下一曲 (切换文件并恢复录制)
!S:: {
    FileAppend("", EnvGet("TEMP") . "\mpv_toggle_next")
}

```

#### **Linux 平台 (Sway/i3):**

```bash
bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
bindsym $mod+n exec touch /tmp/mpv_toggle_next

```

### 2. 启动程序

**模式 A: 配合 GPT-SoVITS 清单 (推荐，支持歌词/字幕显示)**
如果你有 GPT-SoVITS 生成的 list 文件，程序将在系统通知栏显示对应的歌词/字幕。

```bash
om -l "C:\path\to\slicer_opt.list" "C:\path\to\audio_folder"

```

**模式 B: 普通目录模式**
逐个播放文件夹中的所有音频文件。

```bash
om /path/to/your/audio_folder

```

# 命令行参数详解

* `-l, --list <PATH>`: 指定 GPT-SoVITS `.list` 文件的路径。如果未指定，程序会尝试在目标目录中寻找名为 `slicer_opt.list` 的文件。
* `<PATHS>...`: 一个或多个媒体文件或文件夹的路径。

# 配置项

在编译之前，你可以编辑 `src/main.rs` 来设置你的 OBS WebSocket 密码：

```rust
const OBS_PASSWORD: &str = "your_password_here";

```

# 编译方法

```bash
# 编译 Release 版本
cargo build --release

# 可执行文件路径：
# Windows: target/release/om.exe
# Linux: target/release/om

```


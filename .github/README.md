[中文文档](README_CN.md) 

# OBS-MPV Sync Control (om)

> [!NOTE]
> This is a powerful utility, written in Rust, that allows you to synchronously control OBS Studio recording and MPV playback. It is specifically optimized for **Voice-Over recording** and **Content Creation**, featuring one-by-one manual playback and synchronized lyric/subtitles notifications.

# New Features

* **Synchronized Control:** Toggle pause/resume for both OBS and MPV simultaneously via global hotkeys.
* **Manual Stepped Playback:** Instead of auto-playing, it loads the next file only when triggered, staying paused at the end of each clip.
* **GPT-SoVITS Integration:** Automatically parses `slicer_opt.list` files to display lyrics/subtitles as system notifications (Linux `notify-send` / Windows Toast).
* **Smart Path Mapping:** Automatically resolves audio file paths even if the list file contains Windows-style relative paths or is moved.
* **Safe Exit:** Automatically stops OBS recording and cleans up MPV processes upon exit.

# Requirements

* Linux (COSMIC/Sway/X11) or Windows 10/11
* [MPV Player](https://mpv.io/)
* [OBS Studio](https://obsproject.com/) with WebSocket enabled (Optional, not required if you are just using mpv to play videos)
* [obs-cmd](https://github.com/grigio/obs-cmd) (Ensure it's in your system PATH) (Optional, not required if you are just using mpv to play videos)

# How to Use

### 1. Global Hotkeys Setup

You need to set up two hotkeys to create "trigger files" in your system's temp directory.

#### **On Windows (AutoHotkey v2):**

[https://www.autohotkey.com/](https://www.autohotkey.com/) 

```ahk
#Requires AutoHotkey v2.0

; Alt + D: Toggle Pause/Resume
!d:: {
    FileAppend("", EnvGet("TEMP") . "\obs_mpv_toggle_pause")
}

; Alt + S: Next Track (Switch file and Resume recording)
!S:: {
    FileAppend("", EnvGet("TEMP") . "\mpv_toggle_next")
}

```

#### **On Linux (Sway/i3):**

```bash
bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
bindsym $mod+n exec touch /tmp/mpv_toggle_next

```

### 2. Launching the Program

**Mode A: With GPT-SoVITS List (Recommended for Lyrics/Subtitles)**
If you have a list file from GPT-SoVITS, the program will show lyrics/subtitles in system notifications.

```bash
om -l "C:\path\to\slicer_opt.list" "C:\path\to\audio_folder"

```

**Mode B: Simple Directory Mode**
Just play all audio files in a folder one by one.

```bash
om /path/to/your/audio_folder

```

# Detailed CLI Arguments

* `-l, --list <PATH>`: Specify the path to the GPT-SoVITS `.list` file. If not provided, the program looks for `slicer_opt.list` in the target directory.
* `<PATHS>...`: One or more paths to media files or directories.

# Configuration

Before compiling, you can edit `src/main.rs` to set your OBS password:

```rust
const OBS_PASSWORD: &str = "your_password_here";
```

# Compilation

```bash
# Build release version
cargo build --release

# The executable will be at:
# Windows: target/release/om.exe
# Linux: target/release/om

```


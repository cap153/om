[中文文档](README_CN.md) 

> [!NOTE]  
> This is a utility developed in Rust that allows you to use a global hotkey to synchronously control the recording state (pause/resume) of OBS Studio and the playback state (pause/play) of the MPV player. It's ideal for recording tutorials, reaction videos, or any scenario that requires synchronizing a screen recording with media playback.

# Features

*   **One-Click Sync**: Automatically starts OBS recording and MPV playback when the program launches.
*   **Global Control**: Toggle the pause/resume state of both OBS and MPV simultaneously using a single hotkey configured in your window manager (e.g., Sway).
*   **Safe Exit**: Automatically stops the OBS recording and saves the file, while also closing the MPV process upon exit, preventing data loss or zombie processes.
*   **Secure Connection**: Supports password-protected OBS WebSocket connections. Simply modify the password variable in the source code before compiling.

# Prerequisites

*   Linux
*   [obs-studio-with-websockets](https://github.com/obsproject/obs-websocket)
*   [mpv](https://mpv.io/installation/)
*   [obs-cmd](https://github.com/grigio/obs-cmd)

# How to Use

1.  In OBS, navigate to `Tools` -> `WebSocket Server Settings`. Check `Enable WebSocket Server`, then click `Apply` and `OK`. (If you enable authentication, you must edit the `OBS_PASSWORD` value and recompile).
2.  Set up a hotkey to create a temporary signal file at `/tmp/obs_mpv_toggle_pause`. For example, using Sway:

    ```bash
    bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
    ```

3.  Run this program with the path to the media file you want to sync with MPV:

    ```bash
    om /path/to/media.mp3 # Change this to your media path
    ```

4.  After launching the program, OBS will begin recording and MPV will start playing your media file. You can now use the previously configured hotkey to synchronously toggle the state of OBS and MPV.

# Building from Source

1.  Install the Rust toolchain first.
2.  Clone this repository and `cd` into it.
3.  Edit `src/main.rs` in the current directory to configure the OBS WebSocket server password (leave it empty if authentication is disabled).

    ```rust
    const OBS_PASSWORD: &str = "Enter your password here";
    ```    
4.  Compile and Install

    *   Running `cargo install` below will place the executable at `~/.cargo/bin/om`.

    ```bash
    cargo install --path .
    ```

    *   If you only want to build the project, the executable will be located at `target/release/om` in the current directory.

    ```bash
    cargo build --release
    ```

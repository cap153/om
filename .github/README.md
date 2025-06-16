[中文文档](README_CN.md) 

> [!NOTE]  
> This is a handy utility, written in Rust, that allows you to use a single global hotkey to synchronously control the recording state (pause/resume) of OBS Studio and the playback state (pause/play) of the MPV player. It's perfect for recording tutorials, reaction videos, or any scenario that requires syncing screen recording with media playback.

# Features

*   **Synchronized Start:** Automatically starts OBS recording and MPV playback when the program launches.
*   **Global Control:** Toggle the pause/resume state of both OBS and MPV simultaneously with a global hotkey.
*   **Safe Exit:** Automatically stops the OBS recording and saves the file when MPV is closed, preventing data loss or zombie processes.
*   **Secure Connection:** Supports password-protected OBS WebSocket connections. Simply edit the password variable in the source code before compiling.

# Requirements

*   Linux / Windows
*   [OBS Studio](https://obsproject.com/) with the [obs-websocket](https://github.com/obsproject/obs-websocket) plugin installed
*   [MPV](https://mpv.io/installation/)
*   [obs-cmd](https://github.com/grigio/obs-cmd)

# How to Use

1.  In OBS Studio, navigate to `Tools` > `WebSocket Server Settings`. Ensure `Enable WebSocket Server` is checked, then click `Apply` and `OK`. (If you enable authentication, you will need to set the `OBS_PASSWORD` variable to your server password and recompile the program).

2.  Set up a global hotkey to create a temporary "trigger file" at `/tmp/obs_mpv_toggle_pause`.

    1.  **On Linux**, using a window manager like Sway as an example:

        ```bash
        bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
        ```

    2.  **On Windows**, you can use [AutoHotkey](https://www.autohotkey.com/). Below is an example for AutoHotkey v2. (Note: Running multiple scripts simultaneously may cause conflicts. Please close other scripts or merge this into your existing script and reload it).

        ```ahk
        ; Set the hotkey to Alt + ;
        !;::
        {
            ; Get the path to the system's temporary folder.
            TempPath := EnvGet("TEMP")

            ; Construct the full path for the trigger file.
            TriggerFilePath := TempPath . "\obs_mpv_toggle_pause"
            
            ; Create the trigger file (FileAppend creates the file if it doesn't exist).
            FileAppend("", TriggerFilePath)
        }
        ```

        For future customization, here are some common AutoHotkey modifiers:

        *   `!` : Alt key
        *   `#` : Win key (Windows logo key)
        *   `^` : Ctrl key
        *   `+` : Shift key

        Examples:

        *   `^j::` corresponds to Ctrl + J
        *   `+F1::` corresponds to Shift + F1
        *   `^!s::` corresponds to Ctrl + Alt + S
        *   `#space::` corresponds to Win + Space

3.  Run this program with the media file you want to play in MPV:

    ```bash
    om /path/to/your/media.mp3 # Change this to your media file path
    ```

4.  After the program starts, OBS will begin recording, and MPV will play your media file. You can now use the hotkey you set up to synchronously toggle the state of both OBS and MPV.

# Manual Compilation

1.  Install the [Rust toolchain](https://www.rust-lang.org/tools/install) if you haven't already.

2.  Clone this repository and navigate into the directory.

3.  Edit `src/main.rs` to configure your OBS WebSocket password. (Leave the string empty if authentication is disabled).

    ```rust
    const OBS_PASSWORD: &str = "your_password_here";
    ```

4.  Compile and Install:

    *   The following `cargo install` command will compile the program and place the executable at `~/.cargo/bin/om`:

        ```bash
        cargo install --path .
        ```

    *   If you just want to build the executable, it will be located at `target/release/om` in the project directory:

        ```bash
        cargo build --release
        ```

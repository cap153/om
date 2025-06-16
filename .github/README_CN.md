> [!NOTE]  
> 这是一个 Rust 开发的实用工具，它允许你使用一个全局快捷键，同步控制 OBS Studio 的录制状态（暂停/恢复）和 MPV 播放器的播放状态（暂停/播放）。非常适合用于录制教程、反应视频或任何需要将屏幕录制与媒体播放同步的场景。

# 功能特性

* 一键同步：启动程序时，自动开始 OBS 录制和 MPV 播放。
* 全局控制：通过在窗口管理器（如 Sway）中设置的单一快捷键，同时切换 OBS 和 MPV 的暂停/恢复状态。
* 安全退出：结束程序时，会自动停止 OBS 录制并保存文件，同时关闭 MPV 进程，防止数据丢失或产生僵尸进程。
* 安全连接：支持带密码的 OBS WebSocket 连接，只需在编译前修改源码中的密码变量即可。

# 环境需求

* linux
* [obs-studio-with-websockets](https://github.com/obsproject/obs-websocket)
* [mpv](https://mpv.io/installation/)
* [obs-cmd](https://github.com/grigio/obs-cmd)

# 使用方法

1. 打开obs依次点击工具、WebSocket 服务器设置、开启 WebSocket 服务器、应用、确定(如果开启身份认证需要修改OBS_PASSWORD的值为服务器密码重新编译)。
2. 设置一个快捷键创建一个用作信号的临时文件`/tmp/obs_mpv_toggle_pause`，以我使用的sway为例：

    ```bash
    bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
    ```

3. 使用本程序打开想要mpv同步播放的媒体文件：

    ```bash
    om /path/to/media.mp3 # 修改成你的媒体路径
    ```

4. 程序启动后，OBS 会开始录制，MPV 会开始播放你的媒体文件，此时可以使用前面设置快捷键同步切换obs和mpv的状态了。

# 手动编译

1. 提前安装好rust工具链
2. 克隆本仓库并进入
3. 编辑当前目录下的`src/main.rs`配置OBS WebSocket服务器密码(如果没有开启可以留空)

    ```rust
    const OBS_PASSWORD: &str = "在这里填写你的密码";
    ```
    
4. 编译与安装

    * 执行下面的`cargo install`会把可执行文件放到`~/.cargo/bin/om`

    ```bash
    cargo install --path .
    ```

    * 如果只是想构建，将会放到当前目录的`target/release/om`

    ```bash
    cargo build --release
    ```

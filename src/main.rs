// --- 核心依赖 ---
use serde_json::json;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;
use std::path::PathBuf;

// --- 跨平台 IPC 依赖 ---
// 这个库可以智能地在 Unix 上使用 Unix Sockets，在 Windows 上使用 Named Pipes
use interprocess::local_socket::LocalSocketStream;

// --- 在这里设置你的 OBS WebSocket 密码 ---
// 如果 OBS 没有设置密码，请将此设置为空字符串: ""
const OBS_PASSWORD: &str = "";

// --- 这是用于判断快捷键触发的文件名 (不再是完整路径) ---
// 在sway等窗口管理器添加类似快捷键：bindsym $mod+o exec touch /tmp/obs_mpv_toggle_pause
// 在 Windows 上, 你可以使用 AutoHotkey 或其他工具来创建文件 C:\Users\YourUser\AppData\Local\Temp\obs_mpv_toggle_pause
const TRIGGER_FILE_NAME: &str = "obs_mpv_toggle_pause";

// --- IPC Socket 的名字 (不再是完整路径) ---
const IPC_SOCKET_NAME: &str = "mpv.sock";


// --- NEW: 辅助函数，获取平台特定的触发文件完整路径 ---
fn get_trigger_file_path() -> PathBuf {
    env::temp_dir().join(TRIGGER_FILE_NAME)
}

// --- NEW: 辅助函数，获取平台特定的 IPC Socket 路径，用于传递给 mpv 命令行 ---
fn get_ipc_path_for_cli() -> String {
    #[cfg(windows)]
    {
        // 在 Windows 上，命名管道的路径格式是 \\.\pipe\<name>
        format!(r"\\.\pipe\{}", IPC_SOCKET_NAME)
    }
    #[cfg(unix)]
    {
        // 在 Unix 上，我们使用临时目录下的一个文件作为 socket
        env::temp_dir().join(IPC_SOCKET_NAME).to_string_lossy().into_owned()
    }
}

// --- NEW: 辅助函数，获取用于连接的 IPC Socket 名称 ---
// interprocess 库需要稍微不同的格式
fn get_ipc_path_for_connect() -> String {
    #[cfg(windows)]
    {
        // 在 Windows 上，interprocess 只需要管道名
        IPC_SOCKET_NAME.to_string()
    }
    #[cfg(unix)]
    {
        // 在 Unix 上，它需要完整的文件路径
        env::temp_dir().join(IPC_SOCKET_NAME).to_string_lossy().into_owned()
    }
}


// --- 专门用于调用 obs-cmd 的辅助函数 ---
fn run_obs_command(password: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    let mut full_args: Vec<String> = Vec::new();

    if !password.is_empty() {
        full_args.push("--websocket".to_string());
        full_args.push(format!("obsws://localhost:4455/{}", password));
    }

    for arg in args {
        full_args.push(arg.to_string());
    }

    // Command::new 在 Windows 和 Linux 上都会在 PATH 中寻找可执行文件
    Command::new("obs-cmd").args(&full_args).output()
}


// 定义一个结构体来管理 mpv 子进程
struct MpvProcess {
    child: Child,
}

impl MpvProcess {
    // --- CHANGED: 现在使用跨平台辅助函数获取 socket 路径 ---
    fn start(file_path: &str, socket_path_for_cli: &str) -> Result<Self, std::io::Error> {
        let child = Command::new("mpv")
            .arg(file_path)
            .arg(format!("--input-ipc-server={}", socket_path_for_cli))
            .arg("--force-window")
            .arg("--idle=yes")
            .spawn()?;
        Ok(MpvProcess { child })
    }
}

// Drop trait (保持不变，它调用的函数现在是跨平台的)
impl Drop for MpvProcess {
    fn drop(&mut self) {
        println!("正在停止 OBS 录制...");
        if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "stop"]) {
            eprintln!("警告: 停止 OBS 录制失败: {}. 请检查 OBS 是否在运行、密码是否正确。", e);
        } else {
            println!("OBS 录制已停止。");
        }

        println!("正在关闭 mpv...");
        // .kill() 在 Windows 和 Linux 上都能工作
        if let Err(e) = self.child.kill() {
            eprintln!("无法终止 mpv 进程: {}", e);
        }
        self.child.wait().ok();
        println!("mpv 已关闭。");

        // --- NEW: 清理 IPC socket 文件 (在 Unix 上很重要) ---
        #[cfg(unix)]
        {
           fs::remove_file(get_ipc_path_for_cli()).ok();
        }
    }
}

// --- CHANGED: 向 mpv 发送命令的函数，现在使用 interprocess ---
// --- CORRECTED: 向 mpv 发送命令的函数，现在正确处理所有权 ---
fn send_mpv_command(socket_path_for_connect: &str, command: &serde_json::Value) -> Result<String, std::io::Error> {
    // LocalSocketStream::connect 会在 Windows 上连接命名管道，在 Unix 上连接 Unix Socket
    let stream = LocalSocketStream::connect(socket_path_for_connect)?;

    // 创建一个 BufReader 来包装 stream。
    // 注意：这里我们直接传递 `stream`，将所有权移动到 `reader` 中。
    // `reader` 需要是 mut 的，因为 read_line 会修改其内部缓冲区。
    let mut reader = BufReader::new(stream);

    // 准备要发送的命令
    let mut command_str = command.to_string();
    command_str.push('\n');

    // 写入命令
    // 我们通过 reader.get_mut() 获取对内部 stream 的可变引用。
    // 因为 LocalSocketStream 实现了 `Write`，所以我们可以调用 `write_all`。
    reader.get_mut().write_all(command_str.as_bytes())?;

    // 读取响应
    // `reader` 本身实现了 `BufRead`，所以我们可以直接调用 `read_line`。
    let mut response = String::new();
    reader.read_line(&mut response)?;

    Ok(response)
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        // 在 Windows 上，人们可能通过双击运行，提示信息更友好一些
        eprintln!("用法: 请将视频文件拖拽到此程序的 exe 文件上，");
        eprintln!("   或通过命令行运行: {} <视频文件路径>", args[0]);
        // 在 Windows 上，添加一个暂停，以便用户能看到错误信息
        if cfg!(windows) {
            println!("按 Enter 键退出...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        return;
    }
    let video_file = &args[1];

    // --- CHANGED: 使用跨平台辅助函数 ---
    let trigger_file_path = get_trigger_file_path();
    let socket_path_for_cli = get_ipc_path_for_cli();
    let socket_path_for_connect = get_ipc_path_for_connect();
    
    // 清理上一次可能遗留的触发文件和 socket 文件
    fs::remove_file(&trigger_file_path).ok();
    #[cfg(unix)]
    {
        fs::remove_file(&socket_path_for_cli).ok();
    }


    let _mpv_handle = match MpvProcess::start(video_file, &socket_path_for_cli) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("启动 mpv 失败: {e}。请确保 mpv 已安装并在系统的 PATH 环境变量中。");
            return;
        }
    };
    println!("mpv 已启动。播放文件: {}", video_file);
    
    // 等待 mpv 完全启动并创建好 IPC socket/pipe
    thread::sleep(Duration::from_millis(1000));

    println!("正在启动 OBS 录制...");
    if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "start"]) {
        eprintln!("警告: 启动 OBS 录制失败: {e}。请确保 OBS 正在运行，并且 obs-websocket 插件已正确配置（端口、密码等）。");
    } else {
        println!("OBS 已开始录制。");
    }

    println!("快捷键触发方式: 创建文件 '{}'", trigger_file_path.display());
    println!("程序正在后台监听快捷键... 按 Ctrl+C 退出。");
    
    let toggle_pause_command = json!({"command": ["cycle", "pause"]});
    let mut is_paused_state = false;

    loop {
        // --- CHANGED: 使用跨平台路径 ---
        if trigger_file_path.exists() {
            println!("\n快捷键触发！");

            // 1. 控制 mpv
            println!("正在切换 mpv 播放/暂停状态...");
            match send_mpv_command(&socket_path_for_connect, &toggle_pause_command) {
                Ok(response) => println!("-> mpv 响应: {}", response.trim()),
                Err(e) => eprintln!("-> 向 mpv 发送命令失败: {}", e),
            }

            // 2. 控制 OBS (逻辑不变)
            if is_paused_state {
                println!("正在恢复 OBS 录制...");
                if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "resume"]) {
                    eprintln!("-> 恢复 OBS 录制失败: {}", e);
                }
            } else {
                println!("正在暂停 OBS 录制...");
                if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "pause"]) {
                    eprintln!("-> 暂停 OBS 录制失败: {}", e);
                }
            }
            is_paused_state = !is_paused_state;
            
            // --- CHANGED: 使用跨平台路径 ---
            if let Err(e) = fs::remove_file(&trigger_file_path) {
                eprintln!("-> 无法删除触发文件: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

use std::io::{BufRead, BufReader, Write}; // 修正：移除了未使用的 `Read`
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;
use std::{env, fs};
use serde_json::json;

// --- 在这里设置你的 OBS WebSocket 密码 ---
// 如果 OBS 没有设置密码，请将此设置为空字符串: ""
const OBS_PASSWORD: &str = ""; // 请修改为你的密码

// --- 修正：专门用于调用 obs-cmd 的辅助函数 ---
// 现在它会正确地构建 websocket URL 来包含密码
fn run_obs_command(password: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    let mut full_args: Vec<String> = Vec::new();

    // 只有在密码不是空字符串时，才构建包含密码的 websocket URL
    if !password.is_empty() {
        full_args.push("--websocket".to_string());
        full_args.push(format!("obsws://localhost:4455/{}", password));
    }
    // 如果密码是空的，则不添加任何参数，让 obs-cmd 使用默认的无密码连接

    // 添加命令本身的其他参数 (e.g., "recording", "start")
    for arg in args {
        full_args.push(arg.to_string());
    }

    Command::new("obs-cmd").args(&full_args).output()
}


// 定义一个结构体来管理 mpv 子进程
struct MpvProcess {
    child: Child,
}

impl MpvProcess {
    fn start(file_path: &str, socket_path: &str) -> Result<Self, std::io::Error> {
        let child = Command::new("mpv")
            .arg(file_path)
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--force-window")
            .arg("--idle=yes")
            .spawn()?;
        Ok(MpvProcess { child })
    }
}

// Drop trait (保持不变，但现在会调用修正后的函数)
impl Drop for MpvProcess {
    fn drop(&mut self) {
        println!("正在停止 OBS 录制...");
        if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "stop"]) {
            eprintln!("警告: 停止 OBS 录制失败: {}. 请检查 OBS 是否在运行、密码是否正确。", e);
        } else {
            println!("OBS 录制已停止。");
        }

        println!("正在关闭 mpv...");
        if let Err(e) = self.child.kill() {
            eprintln!("无法杀死 mpv 进程: {}", e);
        }
        self.child.wait().ok();
        println!("mpv 已关闭。");
    }
}

// 向 mpv 发送命令的函数 (保持不变)
fn send_mpv_command(socket_path: &str, command: &serde_json::Value) -> Result<String, std::io::Error> {
    let stream = UnixStream::connect(socket_path)?;
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;

    let mut command_str = command.to_string();
    command_str.push('\n');
    writer.write_all(command_str.as_bytes())?;

    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
}


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: {} <视频文件路径>", args[0]);
        return;
    }
    let video_file = &args[1];
    let socket_path = "/tmp/mpv.sock";
    let trigger_file = "/tmp/rust_mpv_toggle_pause";

    fs::remove_file(trigger_file).ok();

    let _mpv_handle = match MpvProcess::start(video_file, socket_path) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("启动 mpv 失败: {e}。请确保 mpv 已安装并在 PATH 中。");
            return;
        }
    };
    println!("mpv 已启动。播放文件: {}", video_file);
    
    thread::sleep(Duration::from_millis(500));

    println!("正在启动 OBS 录制...");
    if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "start"]) {
        eprintln!("警告: 启动 OBS 录制失败: {e}。请确保 OBS 正在运行，并且 obs-websocket 插件已正确配置（端口、密码等）。");
    } else {
        println!("OBS 已开始录制。");
    }

    println!("Sway 快捷键已被设置为触摸 '{}'", trigger_file);
    println!("程序正在后台监听快捷键... 按 Ctrl+C 退出。");
    
    let toggle_pause_command = json!({"command": ["cycle", "pause"]});
    let mut is_paused_state = false;

    loop {
        if fs::metadata(trigger_file).is_ok() {
            println!("\n快捷键触发！");

            // 1. 控制 mpv
            println!("正在切换 mpv 播放/暂停状态...");
            match send_mpv_command(socket_path, &toggle_pause_command) {
                Ok(response) => println!("-> mpv 响应: {}", response.trim()),
                Err(e) => eprintln!("-> 向 mpv 发送命令失败: {}", e),
            }

            // 2. 控制 OBS
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

            if let Err(e) = fs::remove_file(trigger_file) {
                eprintln!("-> 无法删除触发文件: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

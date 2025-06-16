use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;
use std::{env, fs};

// --- 辅助函数：执行外部命令 ---
// 方便我们调用 obs-cmd
fn run_external_command(program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(program).args(args).output()
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

// --- Drop trait (修改后) ---
// 在程序退出时，同时停止 OBS 录制和关闭 mpv
impl Drop for MpvProcess {
    fn drop(&mut self) {
        println!("正在停止 OBS 录制...");
        if let Err(e) = run_external_command("obs-cmd", &["recording", "stop"]) {
            eprintln!(
                "警告: 停止 OBS 录制失败: {}. 可能 OBS 已关闭或 obs-cmd 有问题。",
                e
            );
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
fn send_mpv_command(
    socket_path: &str,
    command: &serde_json::Value,
) -> Result<String, std::io::Error> {
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
    let trigger_file = "/tmp/rust_mpv_toggle_pause"; // 你可以改成你自己的路径
    // 清理旧的触发文件
    fs::remove_file(trigger_file).ok();

    // --- 启动 mpv ---
    let _mpv_handle = match MpvProcess::start(video_file, socket_path) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("启动 mpv 失败: {}。请确保 mpv 已安装并在 PATH 中。", e);
            return;
        }
    };
    println!("mpv 已启动。播放文件: {}", video_file);

    // 等待 mpv 创建 socket
    thread::sleep(Duration::from_millis(500));

    // --- 启动 OBS 录制 ---
    println!("正在启动 OBS 录制...");
    if let Err(e) = run_external_command("obs-cmd", &["recording", "start"]) {
        eprintln!(
            "警告: 启动 OBS 录制失败: {e}。请确保 OBS 正在运行，并且 obs-websocket 插件已正确配置（端口、无密码等）。"
        );
        // 这里我们只打印警告，程序继续运行
    } else {
        println!("OBS 已开始录制。");
    }

    println!("Sway 快捷键已被设置为触摸 '{}'", trigger_file);
    println!("程序正在后台监听快捷键... 按 Ctrl+C 退出。");

    // --- 主循环 (修改后) ---
    let toggle_pause_command = json!({"command": ["cycle", "pause"]});
    let mut is_obs_paused = false; // 新增：OBS 录制暂停状态

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
            if is_obs_paused {
                println!("正在恢复 OBS 录制...");
                if let Err(e) = run_external_command("obs-cmd", &["recording", "resume"]) {
                    eprintln!("-> 恢复 OBS 录制失败: {}", e);
                }
            } else {
                println!("正在暂停 OBS 录制...");
                if let Err(e) = run_external_command("obs-cmd", &["recording", "pause"]) {
                    eprintln!("-> 暂停 OBS 录制失败: {}", e);
                }
            }
            // 切换状态
            is_obs_paused = !is_obs_paused;

            // 清理触发文件
            if let Err(e) = fs::remove_file(trigger_file) {
                eprintln!("-> 无法删除触发文件: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

use serde_json::json;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;

use interprocess::local_socket::LocalSocketStream;

const OBS_PASSWORD: &str = "";
const TRIGGER_PAUSE_NAME: &str = "obs_mpv_toggle_pause";
const TRIGGER_NEXT_NAME: &str = "mpv_toggle_next"; // 新增：下一曲触发文件名
const IPC_SOCKET_NAME: &str = "mpv.sock";

// --- 路径辅助函数 ---
fn get_trigger_path(name: &str) -> PathBuf {
    env::temp_dir().join(name)
}

fn get_ipc_path_for_cli() -> String {
    #[cfg(unix)]
    {
        env::temp_dir()
            .join(IPC_SOCKET_NAME)
            .to_string_lossy()
            .into_owned()
    }
}

fn get_ipc_path_for_connect() -> String {
    #[cfg(unix)]
    {
        env::temp_dir()
            .join(IPC_SOCKET_NAME)
            .to_string_lossy()
            .into_owned()
    }
}

// --- 媒体文件扫描 ---
fn collect_media_files(paths: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    let extensions = ["mp3", "wav", "flac", "mp4", "mkv", "mov"];

    for p in paths {
        let path = Path::new(p);
        if path.is_file() {
            files.push(path.to_string_lossy().into_owned());
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    if ep.is_file()
                        && extensions
                            .iter()
                            .any(|&ext| ep.extension().map_or(false, |e| e == ext))
                    {
                        files.push(ep.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    files.sort(); // 确保播放顺序
    files
}

// --- OBS & MPV 控制 ---
fn run_obs_command(password: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    let mut full_args: Vec<String> = Vec::new();
    if !password.is_empty() {
        full_args.push("--websocket".to_string());
        full_args.push(format!("obsws://localhost:4455/{}", password));
    }
    for arg in args {
        full_args.push(arg.to_string());
    }
    Command::new("obs-cmd").args(&full_args).output()
}

struct MpvProcess {
    child: Child,
}

impl MpvProcess {
    fn start(files: &[String], socket_path: &str) -> Result<Self, std::io::Error> {
        let mut cmd = Command::new("mpv");
        cmd.args(files)
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--force-window")
            .arg("--idle=yes")
            .arg("--keep-open=always") // 每个文件播完都停住
            .arg("--reset-on-next-file=pause") // 切换到新文件时自动暂停
            .arg("--pause=yes"); // 初始暂停

        let child = cmd.spawn()?;
        Ok(MpvProcess { child })
    }

    fn has_exited(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => true,
        }
    }
}

impl Drop for MpvProcess {
    fn drop(&mut self) {
        let _ = run_obs_command(OBS_PASSWORD, &["recording", "stop"]);
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(unix)]
        {
            let _ = fs::remove_file(get_ipc_path_for_cli());
        }
    }
}

fn send_mpv_command(
    socket_path: &str,
    command: &serde_json::Value,
) -> Result<String, std::io::Error> {
    let stream = LocalSocketStream::connect(socket_path)?;
    let mut reader = BufReader::new(stream);
    let mut cmd_str = command.to_string();
    cmd_str.push('\n');
    reader.get_mut().write_all(cmd_str.as_bytes())?;
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        println!(
            "用法: {} <文件或文件夹路径...>",
            env::args().next().unwrap()
        );
        return;
    }

    let media_files = collect_media_files(&args);
    if media_files.is_empty() {
        println!("未找到有效的媒体文件。");
        return;
    }

    let pause_trigger = get_trigger_path(TRIGGER_PAUSE_NAME);
    let next_trigger = get_trigger_path(TRIGGER_NEXT_NAME);
    let socket_cli = get_ipc_path_for_cli();
    let socket_conn = get_ipc_path_for_connect();

    // 清理残留
    let _ = fs::remove_file(&pause_trigger);
    let _ = fs::remove_file(&next_trigger);

    println!("正在启动 OBS 录制...");
    if let Err(e) = run_obs_command(OBS_PASSWORD, &["recording", "start"]) {
        eprintln!("OBS 启动失败: {e}");
        return;
    }

    println!(
        "正在启动 mpv 并加载播放列表 (共 {} 个文件)...",
        media_files.len()
    );
    let mut mpv_handle = MpvProcess::start(&media_files, &socket_cli).expect("mpv 启动失败");

    thread::sleep(Duration::from_millis(1000));
    println!(
        "监听中...\n- 暂停/恢复录制: touch {}\n- 播放下一集: touch {}",
        pause_trigger.display(),
        next_trigger.display()
    );

    let cmd_pause_toggle = json!({"command": ["cycle", "pause"]});
    let cmd_next = json!({"command": ["playlist-next", "force"]});
    let cmd_play = json!({"command": ["set_property", "pause", false]});

    let mut is_paused = true;

    loop {
        if mpv_handle.has_exited() {
            break;
        }

        // 处理 暂停/恢复
        if pause_trigger.exists() {
            let _ = fs::remove_file(&pause_trigger);
            let _ = send_mpv_command(&socket_conn, &cmd_pause_toggle);

            // 这里建议通过 IPC 获取真实 pause 状态，简单处理则取反
            if is_paused {
                let _ = run_obs_command(OBS_PASSWORD, &["recording", "resume"]);
            } else {
                let _ = run_obs_command(OBS_PASSWORD, &["recording", "pause"]);
            }
            is_paused = !is_paused;
        }

        // 处理 下一集：切换并开始
        if next_trigger.exists() {
            let _ = fs::remove_file(&next_trigger);

            // 1. 发送切换指令
            let _ = send_mpv_command(&socket_conn, &cmd_next);
            // 2. 稍微等待 mpv 加载新文件（IPC 响应有微小延迟）
            thread::sleep(Duration::from_millis(50));
            // 3. 发送播放指令 (因为 reset-on-next-file 会让它处于暂停状态)
            let _ = send_mpv_command(&socket_conn, &cmd_play);
            // 4. 同步 OBS
            let _ = run_obs_command(OBS_PASSWORD, &["recording", "resume"]);
            is_paused = false;

            println!(">> 已跳转至下一集并自动开始录制");
        }

        thread::sleep(Duration::from_millis(100));
    }
}

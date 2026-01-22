use serde_json::json;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::thread;
use std::time::Duration;

use interprocess::local_socket::LocalSocketStream;

// --- 常量配置 ---
const OBS_PASSWORD: &str = "";
const TRIGGER_PAUSE_NAME: &str = "obs_mpv_toggle_pause";
const TRIGGER_NEXT_NAME: &str = "mpv_toggle_next";
const IPC_SOCKET_NAME: &str = "mpv.sock";
const DEFAULT_LIST_NAME: &str = "slicer_opt.list";

#[derive(Debug, Clone)]
struct MediaItem {
    path: PathBuf,
    text: String,
}

// --- 1. 跨平台路径与通知辅助 ---

fn get_trigger_path(name: &str) -> PathBuf {
    env::temp_dir().join(name)
}

fn get_ipc_path_for_cli() -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\{}", IPC_SOCKET_NAME)
    }
    #[cfg(unix)]
    {
        env::temp_dir()
            .join(IPC_SOCKET_NAME)
            .to_string_lossy()
            .into_owned()
    }
}

fn get_ipc_path_for_connect() -> String {
    #[cfg(windows)]
    {
        IPC_SOCKET_NAME.to_string()
    }
    #[cfg(unix)]
    {
        env::temp_dir()
            .join(IPC_SOCKET_NAME)
            .to_string_lossy()
            .into_owned()
    }
}

fn show_notification(text: &str) {
    if text.is_empty() {
        return;
    }

    #[cfg(windows)]
    {
        let safe_text = text.replace("'", "''");
        // 标题设为空字符串，在 Windows 中会显得更精简
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $n = New-Object System.Windows.Forms.NotifyIcon; \
             $n.Icon = [System.Drawing.SystemIcons]::Information; \
             $n.Visible = $true; \
             $n.ShowBalloonTip(5000, '', '{}', [System.Windows.Forms.ToolTipIcon]::None)",
            safe_text
        );
        let _ = Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .spawn();
    }

    #[cfg(unix)]
    {
        // Linux 下将标题设为内容，或者设为空字符串
        let _ = Command::new("notify-send")
            .arg("") // 留空标题
            .arg(text)
            .spawn();
    }
}

// --- 2. 优化后的解析逻辑 ---

fn parse_sovits_list(list_path: &Path, search_root: Option<&Path>) -> Vec<MediaItem> {
    let mut items = Vec::new();
    let list_parent = list_path.parent().unwrap_or(Path::new("."));

    if let Ok(file) = fs::File::open(list_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let raw_path_str = parts[0].trim().replace('\\', "/");
                let text = parts[3].trim().to_string();

                let raw_path = Path::new(&raw_path_str);
                let file_name = raw_path.file_name().unwrap_or_default();

                // 路径尝试策略：
                // 1. 尝试 list 文件所在的相对路径
                let mut final_path = list_parent.join(&raw_path_str);

                // 2. 如果不存在，尝试在用户指定的 search_root 里找文件名
                if !final_path.exists() {
                    if let Some(root) = search_root {
                        // 尝试 root + 原始路径后半段
                        final_path = root.join(file_name);

                        // 3. 如果还是不存在，尝试 root + 原始路径全段 (处理相对路径重叠)
                        if !final_path.exists() {
                            final_path = root.join(&raw_path_str);
                        }
                    }
                }

                // 调试输出：如果依然找不到，打印一下。
                if !final_path.exists() {
                    eprintln!(
                        "警告: 无法定位音频文件: {} (尝试路径: {:?})",
                        raw_path_str, final_path
                    );
                }

                items.push(MediaItem {
                    path: final_path,
                    text,
                });
            }
        }
    }
    items
}

// --- 3. 优化后的资源搜集 ---

fn collect_items(args: &[String], list_flag: Option<String>) -> Vec<MediaItem> {
    // 确定搜索根目录（取 args 中第一个存在的目录）
    let search_root = args.iter().map(Path::new).find(|p| p.is_dir());

    // 如果指定了 -l
    if let Some(lp) = list_flag {
        let p = Path::new(&lp);
        if p.exists() {
            return parse_sovits_list(p, search_root);
        }
    }

    // 如果没指定 -l，在 args 目录里搜 slicer_opt.list
    if let Some(root) = search_root {
        let potential_list = root.join(DEFAULT_LIST_NAME);
        if potential_list.exists() {
            return parse_sovits_list(&potential_list, Some(root));
        }
    }

    // 兜底：原来的逻辑，仅扫描媒体文件
    let mut files = Vec::new();
    let exts = ["mp3", "wav", "flac", "mp4", "mkv"];
    for arg in args {
        let p = Path::new(arg);
        if p.is_file() {
            files.push(MediaItem {
                path: p.to_path_buf(),
                text: String::new(),
            });
        } else if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    if ep.is_file()
                        && exts
                            .iter()
                            .any(|&e| ep.extension().map_or(false, |ext| ext == e))
                    {
                        files.push(MediaItem {
                            path: ep,
                            text: String::new(),
                        });
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

// --- 4. 进程与 IPC 控制 ---

fn run_obs_command(password: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    let mut full_args = Vec::new();
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
    fn start(socket_path: &str) -> Result<Self, std::io::Error> {
        let child = Command::new("mpv")
            .arg("--idle=yes")
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--force-window")
            .arg("--keep-open=always") // 播完停在末尾，不自动跳
            .arg("--pause=yes") // 仅初始启动时暂停
            .spawn()?;
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

    let writer = reader.get_mut();
    writer.write_all(cmd_str.as_bytes())?;

    let mut response = String::new();
    let _ = reader.read_line(&mut response);
    Ok(response)
}

// --- 5. 主程序 ---

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut list_param = None;

    // 解析 -l / --list
    if let Some(pos) = args.iter().position(|x| x == "-l" || x == "--list") {
        if pos + 1 < args.len() {
            list_param = Some(args[pos + 1].clone());
            args.remove(pos + 1);
            args.remove(pos);
        }
    }

    let playlist = collect_items(&args, list_param);
    if playlist.is_empty() {
        println!("错误: 未找到任何媒体文件或有效的 list 文件。");
        return;
    }

    let pause_trigger = get_trigger_path(TRIGGER_PAUSE_NAME);
    let next_trigger = get_trigger_path(TRIGGER_NEXT_NAME);
    let socket_cli = get_ipc_path_for_cli();
    let socket_conn = get_ipc_path_for_connect();

    let _ = fs::remove_file(&pause_trigger);
    let _ = fs::remove_file(&next_trigger);

    println!("正在启动 OBS 录制...");
    let _ = run_obs_command(OBS_PASSWORD, &["recording", "start"]);

    println!("正在启动 mpv (Idle模式)...");
    let mut mpv_handle = MpvProcess::start(&socket_cli).expect("mpv 启动失败");
    thread::sleep(Duration::from_millis(800));

    let mut current_idx: Option<usize> = None;
    let mut is_paused = false;

    println!(
        "已加载 {} 个条目。等待触发 next 以开始播放第一条。",
        playlist.len()
    );

    loop {
        if mpv_handle.has_exited() {
            break;
        }

        // 处理 下一首 (核心逻辑)
        if next_trigger.exists() {
            let _ = fs::remove_file(&next_trigger);

            let next_idx = match current_idx {
                None => 0,
                Some(idx) => idx + 1,
            };

            if next_idx < playlist.len() {
                let item = &playlist[next_idx];
                println!(
                    ">> 播放 [{} / {}]: {}",
                    next_idx + 1,
                    playlist.len(),
                    item.text
                );

                // 1. 发送加载指令 (replace 模式)
                let load_cmd =
                    json!({"command": ["loadfile", item.path.to_string_lossy(), "replace"]});
                let _ = send_mpv_command(&socket_conn, &load_cmd);

                // 2. 显式解除暂停
                // 不再只针对第一集，而是每一集切换都强制 set pause no
                // 为了防止 mpv 忽略指令，我们可以在 load 之后紧跟一个 play
                let play_cmd = json!({"command": ["set_property", "pause", false]});
                let _ = send_mpv_command(&socket_conn, &play_cmd);

                // 3. 同步 OBS 录制状态
                // let _ = run_obs_command(OBS_PASSWORD, &["recording", "resume"]);

                // 4. 发送简洁版通知 (无标题)
                show_notification(&item.text);

                current_idx = Some(next_idx);
                is_paused = false; // 状态位同步
            } else {
                println!(">> 已到达播放列表末尾。");
            }
        }

        // 处理 暂停/恢复
        if pause_trigger.exists() {
            let _ = fs::remove_file(&pause_trigger);
            if current_idx.is_some() {
                let _ = send_mpv_command(&socket_conn, &json!({"command": ["cycle", "pause"]}));
                if is_paused {
                    let _ = run_obs_command(OBS_PASSWORD, &["recording", "resume"]);
                    println!(">> 恢复播放");
                } else {
                    let _ = run_obs_command(OBS_PASSWORD, &["recording", "pause"]);
                    println!(">> 暂停播放");
                }
                is_paused = !is_paused;
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

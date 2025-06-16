use serde_json::json;
use std::io::{BufRead, BufReader, Write}; // 修改这一行
use std::os::unix::net::UnixStream;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use std::{env, fs};

// 定义一个结构体来管理 mpv 子进程
// 当这个结构的实例被销毁 (drop) 时，它的 kill() 方法会被调用
struct MpvProcess {
    child: Child,
}

impl MpvProcess {
    // 启动 mpv 进程并返回一个 MpvProcess 实例
    fn start(file_path: &str, socket_path: &str) -> Result<Self, std::io::Error> {
        let child = Command::new("mpv")
            .arg(file_path)
            .arg(format!("--input-ipc-server={}", socket_path))
            .arg("--force-window") // 确保窗口总是出现
            .arg("--idle=yes") // 播放结束后保持打开状态
            .spawn()?; // spawn() 会立即返回，不会等待 mpv 结束

        Ok(MpvProcess { child })
    }
}

// 实现 Drop trait，这是 Rust 的 RAII (资源获取即初始化) 特性
// 当 MpvProcess 离开作用域时，这个方法会自动执行
impl Drop for MpvProcess {
    fn drop(&mut self) {
        println!("正在关闭 mpv...");
        // 尝试优雅地杀死子进程
        if let Err(e) = self.child.kill() {
            eprintln!("无法杀死 mpv 进程: {}", e);
        }
        // 等待进程完全退出
        self.child.wait().ok();
        println!("mpv 已关闭。");
    }
}

// 向 mpv 发送命令的函数
fn send_mpv_command(
    socket_path: &str,
    command: &serde_json::Value,
) -> Result<String, std::io::Error> {
    // 连接到 Unix socket
    let stream = UnixStream::connect(socket_path)?;
    // 使用 BufReader 包装 stream，以便按行读取
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;

    // 将 JSON 命令转换为字符串，并在末尾加上换行符
    let mut command_str = command.to_string();
    command_str.push('\n');

    // 写入命令
    writer.write_all(command_str.as_bytes())?;

    // 读取 mpv 的响应 (关键改动！)
    // 只读取一行，直到遇到换行符为止
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
    let trigger_file = "/tmp/my_app_hotkey_trigger";

    // 启动 mpv 进程
    // 我们把 MpvProcess 实例绑定到一个变量，这样它就不会立即被 drop
    let _mpv_handle = match MpvProcess::start(video_file, socket_path) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("启动 mpv 失败: {}。请确保 mpv 已安装并在 PATH 中。", e);
            return;
        }
    };

    println!("mpv 已启动。播放文件: {}", video_file);
    println!("Sway 快捷键 (例如 Alt+P) 已被设置为触摸 '{}'", trigger_file);
    println!("程序正在后台监听快捷键... 按 Ctrl+C 退出。");

    // 等待一小会儿，确保 mpv 有足够的时间创建 socket 文件
    thread::sleep(Duration::from_millis(500));

    // 定义切换播放/暂停的命令
    let toggle_pause_command = json!({
        "command": ["cycle", "pause"]
    });

    // 主循环，监听快捷键触发
    loop {
        if fs::metadata(trigger_file).is_ok() {
            println!("快捷键触发！正在切换 mpv 播放/暂停状态...");

            match send_mpv_command(socket_path, &toggle_pause_command) {
                // trim() 去掉末尾的换行符，让输出更整洁
                Ok(response) => println!("mpv 响应: {}", response.trim()),
                Err(e) => eprintln!("向 mpv 发送命令失败: {}", e),
            }

            // 清理触发文件
            if let Err(e) = fs::remove_file(trigger_file) {
                eprintln!("无法删除触发文件: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

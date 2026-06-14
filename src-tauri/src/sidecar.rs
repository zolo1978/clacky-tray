//! Ever-Living Ruby Sidecar 进程管理
//!
//! 核心策略：每次启动前杀掉所有残留 Ever-Living 进程，确保应用完全拥有 sidecar。
//! 不复用已有端口——避免进程所有权丢失导致窗口白屏/无法 stop。

use std::process::{Command, Stdio};

pub const DEFAULT_PORT: u16 = 7070;
const SIDECAR_COMMAND: &str = "ever-living";
const LOG_PREFIX: &str = "[Ever-Living]";

/// 找到 Ever-Living sidecar 二进制。
pub fn find_binary() -> Result<String, String> {
    // 1. 环境变量
    if let Ok(p) = std::env::var("EVER_LIVING_BIN").or_else(|_| std::env::var("CLACKY_BIN")) {
        if std::path::Path::new(&p).exists() {
            return Ok(p);
        }
    }

    // 2. App Bundle Resources（打包时放入）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bundle) = exe.parent().and_then(|p| p.parent()) {
            let bundled = bundle.join("Resources").join(SIDECAR_COMMAND);
            if bundled.exists() {
                return Ok(bundled.to_string_lossy().into());
            }
        }
    }

    // 3. 系统路径
    for dir in ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"] {
        let p = std::path::Path::new(dir).join(SIDECAR_COMMAND);
        if p.exists() {
            return Ok(p.to_string_lossy().into());
        }
    }

    // 4. Ruby gem 路径
    if let Ok(home) = std::env::var("HOME") {
        for ruby_ver in ["3.3.0", "3.2.0", "3.1.0", "3.0.0", "2.7.0", "2.6.0"] {
            let p = std::path::Path::new(&home)
                .join(".gem/ruby")
                .join(ruby_ver)
                .join("bin")
                .join(SIDECAR_COMMAND);
            if p.exists() {
                return Ok(p.to_string_lossy().into());
            }
        }
    }

    // 5. which 兜底
    if let Ok(out) = Command::new("which").arg(SIDECAR_COMMAND).output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Ok(s);
        }
    }

    Err("找不到 Ever-Living sidecar。请安装 ever-living 命令。".into())
}

/// 等待端口就绪（返回 HTTP 2xx）
async fn wait_port(port: u16, timeout_secs: u64) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(format!("服务在 {timeout_secs}s 内未在端口 {port} 就绪"));
        }
        match reqwest::get(&url).await {
            Ok(r) if r.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
        }
    }
}

/// 检查进程是否存活（信号 0，不发送实际信号）。
pub fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// 杀掉系统中所有残留的 Ever-Living server 进程（不限于当前 PID 文件记录的）。
/// 返回被杀掉的进程数量。
pub fn kill_all_sidecars() -> usize {
    let mut pids = Vec::new();

    let output = match Command::new("pgrep")
        .args(["-f", "ever-living server"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return 0,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for pid in stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
    {
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }

    if pids.is_empty() {
        return 0;
    }

    eprintln!(
        "{LOG_PREFIX} 发现 {} 个残留 sidecar 进程，正在清理",
        pids.len()
    );
    let mut killed = 0;
    for pid in &pids {
        unsafe { libc::kill(*pid as i32, libc::SIGTERM) };
    }
    // 等待 2s 让它们优雅退出
    std::thread::sleep(std::time::Duration::from_secs(2));

    for pid in &pids {
        if alive(*pid) {
            unsafe { libc::kill(*pid as i32, libc::SIGKILL) };
        }
        killed += 1;
    }
    killed
}

/// 启动 Ever-Living 服务。返回 (pid, port)
pub async fn start(app_data_dir: &std::path::Path) -> Result<(u32, u16), String> {
    let bin = find_binary()?;
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("无法创建应用数据目录 {}: {e}", app_data_dir.display()))?;

    // 杀掉所有残留 sidecar 进程，确保干净启动
    let killed = kill_all_sidecars();
    if killed > 0 {
        eprintln!("{LOG_PREFIX} 已清理 {} 个残留进程", killed);
    }

    // 清理 PID 文件
    cleanup_stale(app_data_dir)?;

    // 日志文件
    let log = app_data_dir.join("sidecar.log");
    let log_file =
        std::fs::File::create(&log).map_err(|e| format!("无法创建日志 {}: {e}", log.display()))?;

    // 标准输出和标准错误都重定向到日志文件
    let log_out = log_file
        .try_clone()
        .map_err(|e| format!("无法复制日志文件句柄: {e}"))?;

    let mut child = Command::new(&bin)
        .arg("server")
        .arg("--port")
        .arg(DEFAULT_PORT.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("启动失败: {e}\n命令: {bin} server --port {DEFAULT_PORT}"))?;

    let pid = child.id();
    save_pid(app_data_dir, pid)?;

    // 后台等待进程
    tauri::async_runtime::spawn(async move {
        match child.wait() {
            Ok(status) => eprintln!("{LOG_PREFIX} Sidecar 退出: {status}"),
            Err(e) => eprintln!("{LOG_PREFIX} Sidecar wait 错误: {e}"),
        }
    });

    // 等待端口就绪（30s 超时，Ruby 冷启动需要更多时间）
    wait_port(DEFAULT_PORT, 30).await?;

    eprintln!("{LOG_PREFIX} Sidecar 已启动 PID={pid} 端口={DEFAULT_PORT}");
    Ok((pid, DEFAULT_PORT))
}

/// 停止服务：先杀 PID 文件记录的进程，再清理所有可能的残留。
pub async fn stop(app_data_dir: &std::path::Path) -> Result<(), String> {
    // 1. 通过 PID 文件杀已知进程
    if let Some(pid) = load_pid(app_data_dir)? {
        if alive(pid) {
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            for _ in 0..6 {
                if !alive(pid) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            if alive(pid) {
                unsafe { libc::kill(pid as i32, libc::SIGKILL) };
                eprintln!("{LOG_PREFIX} Sidecar 强制终止 PID={pid}");
            } else {
                eprintln!("{LOG_PREFIX} Sidecar 已停止 PID={pid}");
            }
        }
        delete_pid(app_data_dir)?;
    }

    // 2. 兜底：杀掉所有残留 sidecar 进程
    kill_all_sidecars();
    Ok(())
}

// ── PID 文件 ──

fn pid_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("sidecar.pid")
}

fn save_pid(dir: &std::path::Path, pid: u32) -> Result<(), String> {
    std::fs::write(pid_path(dir), pid.to_string()).map_err(|e| format!("写入PID: {e}"))
}

fn load_pid(dir: &std::path::Path) -> Result<Option<u32>, String> {
    let p = pid_path(dir);
    if !p.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&p).map_err(|e| format!("读取PID: {e}"))?;
    Ok(s.trim().parse().ok())
}

fn delete_pid(dir: &std::path::Path) -> Result<(), String> {
    let p = pid_path(dir);
    if p.exists() {
        std::fs::remove_file(&p).map_err(|e| format!("删除PID: {e}"))?;
    }
    Ok(())
}

fn cleanup_stale(dir: &std::path::Path) -> Result<(), String> {
    if let Some(pid) = load_pid(dir)? {
        if alive(pid) {
            eprintln!("{LOG_PREFIX} 清理残留 PID={pid}");
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            std::thread::sleep(std::time::Duration::from_secs(1));
            if alive(pid) {
                unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            }
        }
        delete_pid(dir)?;
    }
    Ok(())
}

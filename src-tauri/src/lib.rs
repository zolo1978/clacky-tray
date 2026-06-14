use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_notification::NotificationExt;

mod preferences;
mod sidecar;

const APP_NAME: &str = "Ever-Living";
const MAIN_WINDOW_LABEL: &str = "ever-living";
const LOG_PREFIX: &str = "[Ever-Living]";

struct AppState {
    pid: Mutex<Option<u32>>,
    port: Mutex<u16>,
}

fn app_data_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().expect("无法获取应用数据目录")
}

/// 获取 Ever-Living locale 缩写（"zh" | "en"）
fn ever_living_lang(prefs: &preferences::Preferences) -> &str {
    if prefs.locale.starts_with("zh") {
        "zh"
    } else {
        "en"
    }
}

#[derive(serde::Serialize)]
struct Status {
    running: bool,
    port: u16,
    pid: Option<u32>,
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<Status, String> {
    let port = *state.port.lock().map_err(|e| e.to_string())?;
    let pid = *state.pid.lock().map_err(|e| e.to_string())?;
    let running = pid.is_some_and(sidecar::alive);
    Ok(Status { running, port, pid })
}

#[tauri::command]
async fn restart_service(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let dir = app_data_dir(&app);
    sidecar::stop(&dir).await?;
    let (pid, port) = sidecar::start(&dir).await?;
    *state.pid.lock().map_err(|e| e.to_string())? = Some(pid);
    *state.port.lock().map_err(|e| e.to_string())? = port;
    Ok(format!("http://127.0.0.1:{port}"))
}

#[tauri::command]
async fn notify(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_prefs() -> Result<preferences::Preferences, String> {
    Ok(preferences::load())
}

#[tauri::command]
async fn set_prefs(prefs: preferences::Preferences) -> Result<(), String> {
    preferences::save(&prefs)
}

/// 应用语言设置到已有的 Ever-Living 窗口
#[tauri::command]
async fn apply_lang(app: tauri::AppHandle, locale: String) -> Result<(), String> {
    let lang = if locale.starts_with("zh") { "zh" } else { "en" };
    if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let js = format!(
            r#"localStorage.setItem('clacky-lang','{lang}');location.reload();"#,
            lang = lang
        );
        w.eval(&js).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn open_prefs_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("preferences") {
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(
        &app,
        "preferences",
        WebviewUrl::App("preferences.html".into()),
    )
    .title("偏好设置")
    .inner_size(400.0, 420.0)
    .resizable(false)
    .build()
    .map_err(|e| e.to_string())?;
    let w2 = w.clone();
    w.on_window_event(move |ev| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
            api.prevent_close();
            let _ = w2.hide();
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .manage(AppState { pid: Mutex::new(None), port: Mutex::new(sidecar::DEFAULT_PORT) })
        .invoke_handler(tauri::generate_handler![
            get_status,
            restart_service,
            notify,
            get_prefs,
            set_prefs,
            apply_lang,
            open_prefs_window,
        ])
        .setup(|app| {
            // ── 系统托盘 ──
            let toggle_item = MenuItemBuilder::with_id("toggle", "显示/隐藏").build(app)?;
            let prefs_item = MenuItemBuilder::with_id("prefs", "偏好设置…").build(app)?;
            let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&toggle_item)
                .item(&prefs_item)
                .item(&sep)
                .item(&quit_item)
                .build()?;

            let default_icon = app.default_window_icon()
                .cloned()
                .expect("请在 tauri.conf.json 中配置 bundle.icon");

            let _tray = TrayIconBuilder::new()
                .icon(default_icon)
                .menu(&menu)
                .tooltip(APP_NAME)
                .on_menu_event(|app, ev| match ev.id().as_ref() {
                    "toggle" => {
                        if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    "prefs" => {
                        let a = app.clone();
                        tauri::async_runtime::spawn(async { let _ = open_prefs_window(a).await; });
                    }
                    "quit" => {
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let dir = a.path().app_data_dir().unwrap();
                            let _ = sidecar::stop(&dir).await;
                            a.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, ev| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = ev
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // ── 全局快捷键 ──
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let h = app.handle().clone();
                if h.global_shortcut().register("CmdOrCtrl+Shift+O").is_ok() {
                    eprintln!("{LOG_PREFIX} 快捷键已注册");
                }
            }

            // ── 启动 Ever-Living 然后创建主窗口 ──
            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let dir = app_data_dir(&h);
                match sidecar::start(&dir).await {
                    Ok((pid, port)) => {
                        if let Ok(mut g) = h.state::<AppState>().pid.lock() { *g = Some(pid); }
                        if let Ok(mut g) = h.state::<AppState>().port.lock() { *g = port; }

                        let url: url::Url = format!("http://127.0.0.1:{port}")
                            .parse()
                            .expect("无效的 URL");

                        let lang = ever_living_lang(&preferences::load()).to_string();
                        let init_script = format!(
                            r#"(function(){{localStorage.setItem('clacky-lang','{l}');Object.defineProperty(navigator,'language',{{value:'{l}-CN',configurable:true}});Object.defineProperty(navigator,'languages',{{value:['{l}-CN','{l}'],configurable:true}});window.addEventListener('load',function(){{if(typeof I18n!=='undefined'&&I18n.lang()!=='{l}'){{I18n.setLang('{l}');}}}})}})();"#,
                            l = lang
                        );

                        // 创建 Ever-Living 主窗口
                        match WebviewWindowBuilder::new(&h, MAIN_WINDOW_LABEL, WebviewUrl::External(url.clone()))
                            .title(APP_NAME)
                            .inner_size(1100.0, 750.0)
                            .min_inner_size(800.0, 500.0)
                            .center()
                            .initialization_script(&init_script)
                            .build()
                        {
                            Ok(w) => {
                                let w2 = w.clone();
                                w.on_window_event(move |ev| {
                                    if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                                        api.prevent_close();
                                        let _ = w2.hide();
                                    }
                                });

                                let _ = h.notification().builder()
                                    .title(APP_NAME)
                                    .body(format!("服务就绪 -> 端口 {port}"))
                                    .show();
                            }
                            Err(e) => {
                                eprintln!("{LOG_PREFIX} 创建窗口失败: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{LOG_PREFIX} 启动失败: {e}");
                        let _ = h.notification().builder()
                            .title("Ever-Living 启动失败")
                            .body(e)
                            .show();
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Ever-Living 启动失败")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                eprintln!("{LOG_PREFIX} App 退出，清理 sidecar");
                sidecar::kill_all_sidecars();
            }
        });
}

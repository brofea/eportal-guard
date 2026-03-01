use std::sync::{Arc, Mutex};

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

use crate::autostart;
use crate::config;
use crate::notifier;
use crate::platform;
use crate::web::{SharedState, TUTORIAL_URL};

pub struct TrayHandle {
    _tray: tray_icon::TrayIcon,
    state: Arc<Mutex<SharedState>>,
    config_path: std::path::PathBuf,
    curl_path: std::path::PathBuf,
    exe_path: std::path::PathBuf,
    on_manual_login: Arc<dyn Fn() + Send + Sync>,
    on_exit: Arc<dyn Fn() + Send + Sync>,
    status_id: MenuId,
    login_id: MenuId,
    open_config_id: MenuId,
    tutorial_id: MenuId,
    open_curl_id: MenuId,
    auto_id: MenuId,
    quit_id: MenuId,
}

pub fn start_tray(
    state: Arc<Mutex<SharedState>>,
    config_path: std::path::PathBuf,
    curl_path: std::path::PathBuf,
    exe_path: std::path::PathBuf,
    on_manual_login: Arc<dyn Fn() + Send + Sync>,
    on_exit: Arc<dyn Fn() + Send + Sync>,
) -> Result<TrayHandle, String> {
    let menu = Menu::new();

    let status_item = MenuItem::new("显示状态", true, None);
    let login_item = MenuItem::new("手动登录", true, None);
    let open_config_item = MenuItem::new("打开配置文件", true, None);
    let tutorial_item = MenuItem::new("获取cURL教程", true, None);
    let open_curl_item = MenuItem::new("粘贴cURL", true, None);
    let auto_item = MenuItem::new("切换开机自启", true, None);
    let quit_item = MenuItem::new("退出程序", true, None);

    menu.append(&status_item).map_err(|e| e.to_string())?;
    menu.append(&login_item).map_err(|e| e.to_string())?;
    menu.append(&open_config_item).map_err(|e| e.to_string())?;
    menu.append(&tutorial_item).map_err(|e| e.to_string())?;
    menu.append(&open_curl_item).map_err(|e| e.to_string())?;
    menu.append(&auto_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let status_id = status_item.id().clone();
    let login_id = login_item.id().clone();
    let open_config_id = open_config_item.id().clone();
    let tutorial_id = tutorial_item.id().clone();
    let open_curl_id = open_curl_item.id().clone();
    let auto_id = auto_item.id().clone();
    let quit_id = quit_item.id().clone();

    let icon = make_icon();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("ePortal Guard")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(TrayHandle {
        _tray: tray,
        state,
        config_path,
        curl_path,
        exe_path,
        on_manual_login,
        on_exit,
        status_id,
        login_id,
        open_config_id,
        tutorial_id,
        open_curl_id,
        auto_id,
        quit_id,
    })
}

impl TrayHandle {
    pub fn process_events(&self) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.status_id {
                let s = self.state.lock().map(|v| v.clone()).unwrap_or_default();
                notifier::notify("ePortal Guard 状态", &format!("{} {}", s.status_text, s.last_error));
                continue;
            }
            if event.id == self.login_id {
                (self.on_manual_login)();
                continue;
            }
            if event.id == self.open_config_id {
                let _ = platform::open_path(&self.config_path);
                continue;
            }
            if event.id == self.tutorial_id {
                let _ = platform::open_url(TUTORIAL_URL);
                continue;
            }
            if event.id == self.open_curl_id {
                let _ = platform::open_path(&self.curl_path);
                continue;
            }
            if event.id == self.auto_id {
                let current = autostart::is_enabled(&self.exe_path);
                match autostart::set_enabled(&self.exe_path, !current) {
                    Ok(_) => notifier::notify(
                        "ePortal Guard",
                        if current { "开机自启已关闭" } else { "开机自启已开启" },
                    ),
                    Err(e) => notifier::notify("ePortal Guard", &e),
                }
                continue;
            }
            if event.id == self.quit_id {
                notifier::notify("ePortal Guard", "程序退出");
                (self.on_exit)();
                break;
            }
        }
    }
}

fn make_icon() -> Icon {
    let w = 16;
    let h = 16;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = (x + y) % 2 == 0;
            let (r, g, b) = if on { (28, 132, 255) } else { (18, 48, 88) };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Icon::from_rgba(rgba, w, h).expect("icon create")
}

pub fn run_manual_login(curl_path: &std::path::Path) {
    let cmd = config::read_curl(curl_path).unwrap_or_default();
    if cmd.trim().is_empty() {
        notifier::notify("ePortal Guard", "curl.txt 为空");
        return;
    }

    if !crate::network::curl_exists() {
        notifier::notify("ePortal Guard", "未检测到系统 curl 命令");
        return;
    }

    let ok = platform::shell_run(&cmd);
    if ok {
        notifier::notify("ePortal Guard", "成功登录");
    } else {
        notifier::notify("ePortal Guard", "登录命令执行失败");
    }
}

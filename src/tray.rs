use std::io::Cursor;
use std::sync::Arc;

use tray_icon::menu::{Icon as MenuIcon, IconMenuItem, Menu, MenuEvent};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

use crate::debuglog;
use crate::platform;

pub struct TrayHandle {
    _tray: tray_icon::TrayIcon,
    panel_url: String,
    on_exit: Arc<dyn Fn() + Send + Sync>,
    open_panel_item: IconMenuItem,
    quit_item: IconMenuItem,
}

pub fn start_tray(
    web_port: u16,
    on_exit: Arc<dyn Fn() + Send + Sync>,
) -> Result<TrayHandle, String> {
    debuglog::log("tray", "start_tray begin");
    let menu = Menu::new();
    let panel_url = format!("http://127.0.0.1:{}/", web_port);

    let open_panel_item = IconMenuItem::new(
        "控制面板",
        true,
        Some(load_menu_icon_bolt()),
        None,
    );
    let quit_item = IconMenuItem::new("退出程序", true, Some(load_menu_icon_logout()), None);

    menu.append(&open_panel_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("ePortal Guard")
        .with_temp_dir_path(crate::paths::app_config_dir())
        .with_menu_on_left_click(true);

    builder = builder.with_icon(load_tray_icon());

    #[cfg(target_os = "macos")]
    {
        builder = builder.with_icon_as_template(true);
        debuglog::log("tray", "macOS tray icon+title mode enabled: template icon");
    }

    let tray = builder
        .build()
        .map_err(|e| {
            debuglog::log("tray", &format!("TrayIconBuilder build failed: {}", e));
            e.to_string()
        })?;
    debuglog::log("tray", "tray icon build success");

    if let Err(e) = tray.set_visible(true) {
        debuglog::log("tray", &format!("set_visible(true) failed: {}", e));
    } else {
        debuglog::log("tray", "set_visible(true) ok");
    }

    Ok(TrayHandle {
        _tray: tray,
        panel_url,
        on_exit,
        open_panel_item,
        quit_item,
    })
}

impl TrayHandle {
    #[cfg(target_os = "macos")]
    pub fn install_macos_event_handlers(&self) {
        let open_panel_id = self.open_panel_item.id().clone();
        let quit_id = self.quit_item.id().clone();

        let panel_url = self.panel_url.clone();
        let on_exit = Arc::clone(&self.on_exit);

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let id = event.id;
            if id == open_panel_id {
                debuglog::log("tray", "click: 打开控制面板");
                let _ = platform::open_url(&panel_url);
                return;
            }
            if id == quit_id {
                debuglog::log("tray", "click: 退出程序");
                on_exit();
            }
        }));

        TrayIconEvent::set_event_handler(Some(move |event| {
            debuglog::log("tray", &format!("tray icon event: {:?}", event));
        }));

        debuglog::log("tray", "macOS menu/tray event handlers installed");
    }

    pub fn process_events(&self) {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            debuglog::log("tray", &format!("tray icon event: {:?}", event));
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            debuglog::log("tray", &format!("menu event id={:?}", event.id));
            if self.handle_event(event.id) {
                break;
            }
        }
    }

    fn handle_event(&self, id: tray_icon::menu::MenuId) -> bool {
        if id == self.open_panel_item.id() {
            debuglog::log("tray", "click: 打开控制面板");
            let _ = platform::open_url(&self.panel_url);
            return false;
        }
        if id == self.quit_item.id() {
            debuglog::log("tray", "click: 退出程序");
            (self.on_exit)();
            return true;
        }
        false
    }
}

fn load_tray_icon() -> Icon {
    match decode_png_rgba(include_bytes!("assets/earth.png")) {
        Ok((rgba, w, h)) => match Icon::from_rgba(rgba, w, h) {
            Ok(icon) => {
                debuglog::log("tray", "tray icon loaded from embedded earth.png");
                icon
            }
            Err(e) => {
                debuglog::log("tray", &format!("globe icon create failed: {}", e));
                make_dot_tray_icon()
            }
        },
        Err(e) => {
            debuglog::log("tray", &format!("embedded earth.png decode failed: {}", e));
            make_dot_tray_icon()
        }
    }
}

fn load_menu_icon_bolt() -> MenuIcon {
    match decode_png_rgba(include_bytes!("assets/bolt.png")) {
        Ok((rgba, w, h)) => match MenuIcon::from_rgba(rgba, w, h) {
            Ok(icon) => {
                debuglog::log("tray", "menu icon loaded from embedded bolt.png");
                icon
            }
            Err(e) => {
                debuglog::log("tray", &format!("bolt icon create failed: {}", e));
                MenuIcon::from_rgba(vec![0; 4], 1, 1).expect("fallback menu icon create")
            }
        },
        Err(e) => {
            debuglog::log("tray", &format!("embedded bolt.png decode failed: {}", e));
            MenuIcon::from_rgba(vec![0; 4], 1, 1).expect("fallback menu icon create")
        }
    }
}

fn load_menu_icon_logout() -> MenuIcon {
    match decode_png_rgba(include_bytes!("assets/log-out.png")) {
        Ok((rgba, w, h)) => match MenuIcon::from_rgba(rgba, w, h) {
            Ok(icon) => {
                debuglog::log("tray", "menu icon loaded from embedded log-out.png");
                icon
            }
            Err(e) => {
                debuglog::log("tray", &format!("log-out icon create failed: {}", e));
                MenuIcon::from_rgba(vec![0; 4], 1, 1).expect("fallback menu icon create")
            }
        },
        Err(e) => {
            debuglog::log("tray", &format!("embedded log-out.png decode failed: {}", e));
            MenuIcon::from_rgba(vec![0; 4], 1, 1).expect("fallback menu icon create")
        }
    }
}

fn decode_png_rgba(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let frame = &buf[..info.buffer_size()];

    let mut rgba = Vec::with_capacity((info.width * info.height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(frame),
        png::ColorType::Rgb => {
            for chunk in frame.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        _ => {
            return Err(format!(
                "unsupported png color type: {:?}",
                info.color_type
            ));
        }
    }

    Ok((rgba, info.width, info.height))
}


fn make_dot_tray_icon() -> Icon {
    let w = 16;
    let h = 16;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let in_dot = (x - 8) * (x - 8) + (y - 8) * (y - 8) <= 36;
            if in_dot {
                rgba.extend_from_slice(&[32, 122, 214, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, w, h).expect("fallback icon create")
}

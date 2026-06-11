#[cfg(target_os = "macos")]
pub fn notify(summary: &str, body: &str) {
    // 使用 notify-rust 的 macOS 原生通知后端，避免启动额外子进程。
    let summary = summary.to_string();
    let body = body.to_string();
    let result = std::panic::catch_unwind(move || {
        notify_rust::Notification::new()
            .appname("ePortal Guard")
            .summary(&summary)
            .body(&body)
            .show()
    });

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => crate::debuglog::log("通知", &format!("发送系统通知失败: {}", e)),
        Err(_) => crate::debuglog::log("通知", "发送系统通知时发生 panic"),
    }
}

#[cfg(target_os = "windows")]
const WINDOWS_APP_ID: &str = "brofea.eportal_guard";

#[cfg(target_os = "windows")]
pub fn notify(summary: &str, body: &str) {
    // Windows Toast 需要应用名和 AppUserModelID，否则通知可能被归到 PowerShell 等宿主进程名下。
    if let Err(e) = ensure_windows_app_id_registered() {
        crate::debuglog::log("通知", &format!("注册 Windows 通知应用 ID 失败: {}", e));
    }

    let summary = summary.to_string();
    let body = body.to_string();
    let result = std::panic::catch_unwind(move || {
        notify_rust::Notification::new()
            .appname("ePortal Guard")
            .app_id(WINDOWS_APP_ID)
            .summary(&summary)
            .body(&body)
            .show()
    });

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => crate::debuglog::log("通知", &format!("发送系统通知失败: {}", e)),
        Err(_) => crate::debuglog::log("通知", "发送系统通知时发生 panic"),
    }
}

#[cfg(target_os = "windows")]
fn ensure_windows_app_id_registered() -> Result<(), String> {
    // 未打包安装的 Win32 程序需要把 AUMID 注册到 HKCU，Toast 才能显示正确应用来源。
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};

    type Hkey = *mut core::ffi::c_void;
    const HKEY_CURRENT_USER: Hkey = 0x80000001usize as Hkey;
    const ERROR_SUCCESS: i32 = 0;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_OPTION_NON_VOLATILE: u32 = 0;
    const REG_SZ: u32 = 1;

    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            h_key: Hkey,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *mut u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const core::ffi::c_void,
            phk_result: *mut Hkey,
            lpdw_disposition: *mut u32,
        ) -> i32;

        fn RegSetValueExW(
            h_key: Hkey,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;

        fn RegCloseKey(h_key: Hkey) -> i32;
    }

    fn wide(input: &str) -> Vec<u16> {
        OsStr::new(input).encode_wide().chain(Some(0)).collect()
    }

    fn set_reg_string(
        key: Hkey,
        name: &str,
        value: &str,
        reg_set_value_ex_w: unsafe extern "system" fn(
            Hkey,
            *const u16,
            u32,
            u32,
            *const u8,
            u32,
        ) -> i32,
    ) -> Result<(), String> {
        let name = wide(name);
        let value = wide(value);
        let bytes = value.len() * std::mem::size_of::<u16>();
        let status = unsafe {
            reg_set_value_ex_w(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast::<u8>(),
                bytes as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("RegSetValueExW 返回 {}", status))
        }
    }

    let subkey = wide(&format!(
        r"SOFTWARE\Classes\AppUserModelId\{}",
        WINDOWS_APP_ID
    ));
    let mut key: Hkey = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("RegCreateKeyExW 返回 {}", status));
    }

    let display_name_result = set_reg_string(key, "DisplayName", "ePortal Guard", RegSetValueExW);
    let background_result = set_reg_string(key, "IconBackgroundColor", "0", RegSetValueExW);
    unsafe {
        RegCloseKey(key);
    }

    display_name_result?;
    background_result?;
    Ok(())
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
pub fn notify(summary: &str, body: &str) {
    // Linux 等平台保留 notify-rust，并用 catch_unwind 避免通知库 panic 影响主流程。
    let summary = summary.to_string();
    let body = body.to_string();
    let result = std::panic::catch_unwind(move || {
        notify_rust::Notification::new()
            .appname("ePortal Guard")
            .summary(&summary)
            .body(&body)
            .show()
    });

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => crate::debuglog::log("通知", &format!("发送系统通知失败: {}", e)),
        Err(_) => crate::debuglog::log("通知", "发送系统通知时发生 panic"),
    }
}

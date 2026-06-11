pub fn open_url(url: &str) -> bool {
    // 用各平台的系统默认方式打开 URL，保持 App 不绑定特定浏览器。
    #[cfg(target_os = "windows")]
    {
        return windows_open_url(url);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_open_url(url);
    }

    #[cfg(target_os = "linux")]
    {
        crate::debuglog::log(
            "平台",
            "Linux 打开 URL 需要桌面门户支持，当前版本未调用外部命令",
        );
        let _ = url;
        return false;
    }

    #[allow(unreachable_code)]
    false
}

#[cfg(target_os = "windows")]
fn windows_open_url(url: &str) -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type Hwnd = *mut core::ffi::c_void;
    type Hinstance = *mut core::ffi::c_void;

    #[link(name = "Shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: Hwnd,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> Hinstance;
    }

    fn wide(input: &str) -> Vec<u16> {
        OsStr::new(input).encode_wide().chain(Some(0)).collect()
    }

    let operation = wide("open");
    let file = wide(url);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    (result as isize) > 32
}

#[cfg(target_os = "macos")]
fn macos_open_url(url: &str) -> bool {
    use std::ffi::c_void;

    type CFAllocatorRef = *const c_void;
    type CFStringEncoding = u32;
    type CFURLRef = *const c_void;
    type OSStatus = i32;

    const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x0800_0100;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateWithBytes(
            allocator: CFAllocatorRef,
            url_bytes: *const u8,
            length: isize,
            encoding: CFStringEncoding,
            base_url: CFURLRef,
        ) -> CFURLRef;
        fn CFRelease(cf: *const c_void);
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn LSOpenCFURLRef(url: CFURLRef, out_launched_url: *mut CFURLRef) -> OSStatus;
    }

    let cf_url = unsafe {
        CFURLCreateWithBytes(
            std::ptr::null(),
            url.as_bytes().as_ptr(),
            url.len() as isize,
            K_CF_STRING_ENCODING_UTF8,
            std::ptr::null(),
        )
    };
    if cf_url.is_null() {
        return false;
    }

    let status = unsafe { LSOpenCFURLRef(cf_url, std::ptr::null_mut()) };
    unsafe {
        CFRelease(cf_url);
    }
    status == 0
}

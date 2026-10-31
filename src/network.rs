use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::time::Instant;

use reqwest::blocking::Client;
use reqwest::redirect::Policy;

// 两个大厂探针都不可达时，才认为互联网不可达，降低单点误判概率。
pub const MIUI_204_URL: &str = "http://connect.rom.miui.com/generate_204";
pub const HUAWEI_204_URL: &str = "http://connectivitycheck.platform.hicloud.com/generate_204";

#[derive(Clone, Debug)]
pub struct HeadProbe {
    pub ok: bool,
    pub status_code: u16,
    pub exit_code: Option<i32>,
    pub error_message: String,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug)]
pub struct InternetProbe {
    pub ok: bool,
    pub miui: HeadProbe,
    pub huawei: HeadProbe,
}

pub fn internet_probe() -> InternetProbe {
    // 小米探针必须返回 204 才算成功；如果被认证页劫持成 200/302，不能误判为外网可达。
    let miui = head_probe(MIUI_204_URL, generate_204_status_ok);
    // 华为探针同样使用 generate_204 语义，只有真实 204 才算外网可达。
    let huawei = head_probe(HUAWEI_204_URL, generate_204_status_ok);
    InternetProbe {
        ok: miui.ok || huawei.ok,
        miui,
        huawei,
    }
}

pub fn head_probe(url: &str, status_ok: impl Fn(u16) -> bool) -> HeadProbe {
    head_probe_with_args(url, status_ok)
}

fn head_probe_with_args(url: &str, status_ok: impl Fn(u16) -> bool) -> HeadProbe {
    let begin = Instant::now();
    // 使用 Rust HTTP 客户端发 HEAD，避免依赖外部命令或弹出命令行窗口。
    // --noproxy '*' 防止系统代理影响探针结果；校园网连通性应该按直连链路判断。
    let result = Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .redirect(Policy::none())
        .no_proxy()
        .build()
        .and_then(|client| client.head(url).send());

    let (process_ok, exit_code, status_code, error_message) = match result {
        Ok(response) => (true, None, response.status().as_u16(), String::new()),
        Err(e) => (false, None, 0, e.to_string()),
    };
    let ok = process_ok && status_ok(status_code);

    HeadProbe {
        ok,
        status_code,
        exit_code,
        error_message,
        elapsed_ms: begin.elapsed().as_millis(),
    }
}

pub fn has_private_ip() -> bool {
    // 开着 VPN（Shadowrocket/Clash 等 TUN 模式）时默认出口是虚拟网卡（如 198.18.0.1），
    // 只看默认出口会误判“未连接内网”而永远不登录；所以先检查所有已启用网卡。
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(found) = any_interface_has_private_ip() {
        return found;
    }
    default_route_ip_is_private()
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn any_interface_has_private_ip() -> Option<bool> {
    use std::ffi::c_void;

    // struct ifaddrs 在 macOS 和 Linux 上前几个字段布局一致（Linux 的 ifa_ifu 是同大小的 union）。
    #[repr(C)]
    struct IfAddrs {
        ifa_next: *mut IfAddrs,
        ifa_name: *const i8,
        ifa_flags: u32,
        ifa_addr: *const u8,
        ifa_netmask: *const c_void,
        ifa_dstaddr: *const c_void,
        ifa_data: *const c_void,
    }

    unsafe extern "C" {
        fn getifaddrs(ifap: *mut *mut IfAddrs) -> i32;
        fn freeifaddrs(ifa: *mut IfAddrs);
    }

    const AF_INET: u16 = 2;
    const IFF_UP: u32 = 0x1;
    const IFF_LOOPBACK: u32 = 0x8;

    let mut head: *mut IfAddrs = std::ptr::null_mut();
    if unsafe { getifaddrs(&mut head) } != 0 {
        return None;
    }

    let mut found = false;
    let mut cur = head;
    while !cur.is_null() {
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_addr.is_null()
            || ifa.ifa_flags & IFF_UP == 0
            || ifa.ifa_flags & IFF_LOOPBACK != 0
        {
            continue;
        }
        // sockaddr 的地址族字段：macOS 是第 2 个字节（前面有 sa_len），Linux 是开头的 u16。
        #[cfg(target_os = "macos")]
        let family = unsafe { *ifa.ifa_addr.add(1) } as u16;
        #[cfg(target_os = "linux")]
        let family = unsafe { (ifa.ifa_addr as *const u16).read_unaligned() };
        if family != AF_INET {
            continue;
        }
        // sockaddr_in 的 IPv4 地址在两个平台上都位于偏移 4。
        let octets = unsafe { (ifa.ifa_addr.add(4) as *const [u8; 4]).read_unaligned() };
        if is_private_v4(Ipv4Addr::from(octets)) {
            found = true;
            break;
        }
    }

    unsafe { freeifaddrs(head) };
    Some(found)
}

fn default_route_ip_is_private() -> bool {
    // 通过 UDP connect 推断当前默认出口 IP，不真正发送数据包。
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return false,
    };
    if socket.connect("8.8.8.8:80").is_err() {
        return false;
    }
    let local = match socket.local_addr() {
        Ok(addr) => addr.ip(),
        Err(_) => return false,
    };

    match local {
        IpAddr::V4(ip) => is_private_v4(ip),
        IpAddr::V6(_) => false,
    }
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    // 校园网通常会分配 RFC1918 私网地址；这里只把 IPv4 私网视作“连接内网”。
    let [a, b, _, _] = ip.octets();
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

fn generate_204_status_ok(code: u16) -> bool {
    code == 204
}

#[cfg(test)]
mod tests {
    use super::generate_204_status_ok;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn interface_scan_runs() {
        assert!(super::any_interface_has_private_ip().is_some());
    }

    // 需要真实连着内网（可同时开 VPN）才成立：cargo test -- --ignored
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    #[ignore]
    fn detects_intranet_even_when_vpn_owns_default_route() {
        assert!(super::has_private_ip());
    }

    #[test]
    fn generate_204_probe_requires_204_to_avoid_captive_portal_false_positive() {
        assert!(generate_204_status_ok(204));
        assert!(!generate_204_status_ok(200));
        assert!(!generate_204_status_ok(302));
        assert!(!generate_204_status_ok(0));
    }
}

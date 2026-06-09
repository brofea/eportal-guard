use std::net::{IpAddr, Ipv4Addr, UdpSocket};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::Instant;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    // 复用系统 curl 发 HEAD，避免引入 HTTP 客户端依赖，也贴近用户登录命令环境。
    // --noproxy '*' 防止系统代理影响探针结果；校园网连通性应该按直连链路判断。
    let output_path = if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    };
    let mut args = vec![
        "--head",
        "--silent",
        "--show-error",
        "--max-time",
        "3",
        "--noproxy",
        "*",
        "--output",
        output_path,
        "--write-out",
        "%{http_code}",
    ];
    args.push(url);

    let mut command = Command::new("curl");
    command.args(args);
    let output = hide_window(&mut command).output();

    let (process_ok, exit_code, status_code, error_message) = match output {
        Ok(output) => {
            let status_code = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u16>()
                .unwrap_or(0);
            let error_message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (
                output.status.success(),
                output.status.code(),
                status_code,
                error_message,
            )
        }
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

pub fn curl_exists() -> bool {
    let mut command = Command::new("curl");
    command.arg("--version");
    hide_window(&mut command)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn hide_window(command: &mut Command) -> &mut Command {
    // 探针和 curl --version 都是后台命令，Windows 下必须禁用控制台窗口闪现。
    command.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(target_os = "windows"))]
fn hide_window(command: &mut Command) -> &mut Command {
    command
}

fn generate_204_status_ok(code: u16) -> bool {
    code == 204
}

#[cfg(test)]
mod tests {
    use super::generate_204_status_ok;

    #[test]
    fn generate_204_probe_requires_204_to_avoid_captive_portal_false_positive() {
        assert!(generate_204_status_ok(204));
        assert!(!generate_204_status_ok(200));
        assert!(!generate_204_status_ok(302));
        assert!(!generate_204_status_ok(0));
    }
}

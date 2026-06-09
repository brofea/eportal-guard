use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::process::Command;
use std::process::Stdio;
use std::time::Instant;

pub const MIUI_204_URL: &str = "http://connect.rom.miui.com/generate_204";
pub const ALIDNS_URL: &str = "https://dns.alicdn.com";

#[derive(Clone, Debug)]
pub struct HeadProbe {
    pub ok: bool,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug)]
pub struct InternetProbe {
    pub ok: bool,
    pub miui: HeadProbe,
    pub alidns: HeadProbe,
}

pub fn internet_probe() -> InternetProbe {
    let miui = head_probe(MIUI_204_URL);
    let alidns = head_probe(ALIDNS_URL);
    InternetProbe {
        ok: miui.ok || alidns.ok,
        miui,
        alidns,
    }
}

pub fn head_probe(url: &str) -> HeadProbe {
    let begin = Instant::now();
    let ok = Command::new("curl")
        .args([
            "--head",
            "--silent",
            "--show-error",
            "--max-time",
            "3",
            "--output",
            #[cfg(target_os = "windows")]
            "NUL",
            #[cfg(not(target_os = "windows"))]
            "/dev/null",
            url,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    HeadProbe {
        ok,
        elapsed_ms: begin.elapsed().as_millis(),
    }
}

pub fn has_private_ip() -> bool {
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
    let [a, b, _, _] = ip.octets();
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

pub fn curl_exists() -> bool {
    Command::new("curl")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

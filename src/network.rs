use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::process::Command;
use std::process::Stdio;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct PingProbe {
    pub ok: bool,
    pub elapsed_ms: u128,
}

pub fn ping_probe(host: &str) -> PingProbe {
    let begin = Instant::now();

    #[cfg(target_os = "windows")]
    {
        let ok = Command::new("ping")
            .args(["-n", "1", "-w", "1000", host])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return PingProbe {
            ok,
            elapsed_ms: begin.elapsed().as_millis(),
        };
    }

    #[cfg(target_os = "macos")]
    {
        let ok = Command::new("ping")
            .args(["-c", "1", "-W", "1000", host])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return PingProbe {
            ok,
            elapsed_ms: begin.elapsed().as_millis(),
        };
    }

    #[cfg(target_os = "linux")]
    {
        let ok = Command::new("ping")
            .args(["-c", "1", "-W", "1", host])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        return PingProbe {
            ok,
            elapsed_ms: begin.elapsed().as_millis(),
        };
    }

    #[allow(unreachable_code)]
    PingProbe {
        ok: false,
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

use std::time::Duration;

use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;

#[derive(Clone, Debug)]
pub struct LoginResult {
    pub method: String,
    pub url: String,
    pub status: u16,
}

#[derive(Clone, Debug)]
struct ParsedCurl {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    accept_invalid_certs: bool,
}

pub fn send_curl_request(command: &str) -> Result<LoginResult, String> {
    let parsed = parse_curl(command)?;
    let method = Method::from_bytes(parsed.method.as_bytes())
        .map_err(|e| format!("HTTP 方法无效: {}", e))?;
    let headers = build_headers(&parsed.headers)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(Policy::limited(10))
        .no_proxy()
        .danger_accept_invalid_certs(parsed.accept_invalid_certs)
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let mut request = client.request(method, &parsed.url).headers(headers);
    if let Some(body) = parsed.body {
        request = request.body(body);
    }

    let response = request
        .send()
        .map_err(|e| format!("发送 HTTP 请求失败: {}", e))?;

    Ok(LoginResult {
        method: parsed.method,
        url: parsed.url,
        status: response.status().as_u16(),
    })
}

fn build_headers(headers: &[(String, String)]) -> Result<HeaderMap, String> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| format!("请求头名称无效 {}: {}", name, e))?;
        let header_value =
            HeaderValue::from_str(value).map_err(|e| format!("请求头值无效 {}: {}", name, e))?;
        map.insert(header_name, header_value);
    }
    Ok(map)
}

fn parse_curl(command: &str) -> Result<ParsedCurl, String> {
    let tokens = shell_words(command)?;
    if tokens.is_empty() {
        return Err("cURL 内容为空".to_string());
    }

    let mut index = if tokens[0] == "curl" || tokens[0] == "curl.exe" {
        1
    } else {
        return Err("请粘贴以 curl 开头的 bash 格式命令".to_string());
    };

    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers = Vec::new();
    let mut body_parts: Vec<Vec<u8>> = Vec::new();
    let mut use_body_as_query = false;
    let mut accept_invalid_certs = false;

    while index < tokens.len() {
        let token = &tokens[index];
        match token.as_str() {
            "-X" | "--request" => {
                method = Some(next_value(&tokens, &mut index, token)?.to_ascii_uppercase());
            }
            value if value.starts_with("-X") && value.len() > 2 => {
                method = Some(value[2..].to_ascii_uppercase());
            }
            "-H" | "--header" => {
                let header = next_value(&tokens, &mut index, token)?;
                push_header(&mut headers, &header)?;
            }
            value if value.starts_with("-H") && value.len() > 2 => {
                push_header(&mut headers, &value[2..])?;
            }
            "-A" | "--user-agent" => {
                let value = next_value(&tokens, &mut index, token)?;
                headers.push(("User-Agent".to_string(), value));
            }
            "-e" | "--referer" => {
                let value = next_value(&tokens, &mut index, token)?;
                headers.push(("Referer".to_string(), value));
            }
            "-b" | "--cookie" => {
                let value = next_value(&tokens, &mut index, token)?;
                headers.push(("Cookie".to_string(), value));
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
                let value = next_value(&tokens, &mut index, token)?;
                push_body_part(&mut body_parts, &value)?;
            }
            value if value.starts_with("--data=") => {
                push_body_part(&mut body_parts, &value["--data=".len()..])?;
            }
            value if value.starts_with("--data-raw=") => {
                push_body_part(&mut body_parts, &value["--data-raw=".len()..])?;
            }
            value if value.starts_with("--data-binary=") => {
                push_body_part(&mut body_parts, &value["--data-binary=".len()..])?;
            }
            "-I" | "--head" => {
                method = Some("HEAD".to_string());
            }
            "-G" | "--get" => {
                use_body_as_query = true;
            }
            "-k" | "--insecure" => {
                accept_invalid_certs = true;
            }
            "--url" => {
                url = Some(next_value(&tokens, &mut index, token)?);
            }
            "--compressed"
            | "-s"
            | "-S"
            | "-i"
            | "--include"
            | "--location"
            | "-L"
            | "--no-progress-meter" => {}
            "--connect-timeout" | "--max-time" | "-m" | "-o" | "--output" => {
                let _ = next_value(&tokens, &mut index, token)?;
            }
            value if value.starts_with('-') => {
                return Err(format!("暂不支持的 cURL 参数: {}", value));
            }
            value => {
                if url.is_none() {
                    url = Some(value.to_string());
                }
            }
        }
        index += 1;
    }

    let mut url = url.ok_or_else(|| "cURL 命令缺少 URL".to_string())?;
    let body = if body_parts.is_empty() {
        None
    } else {
        let joined = join_body_parts(body_parts);
        if use_body_as_query {
            append_query(&mut url, &String::from_utf8_lossy(&joined));
            None
        } else {
            Some(joined)
        }
    };
    let method = method.unwrap_or_else(|| {
        if body.is_some() {
            "POST".to_string()
        } else {
            "GET".to_string()
        }
    });

    Ok(ParsedCurl {
        method,
        url,
        headers,
        body,
        accept_invalid_certs,
    })
}

fn push_header(headers: &mut Vec<(String, String)>, raw: &str) -> Result<(), String> {
    let Some((name, value)) = raw.split_once(':') else {
        return Err(format!("请求头格式无效: {}", raw));
    };
    headers.push((name.trim().to_string(), value.trim().to_string()));
    Ok(())
}

fn push_body_part(parts: &mut Vec<Vec<u8>>, value: &str) -> Result<(), String> {
    if value.starts_with('@') {
        return Err("暂不支持从文件读取请求体".to_string());
    }
    parts.push(value.as_bytes().to_vec());
    Ok(())
}

fn join_body_parts(parts: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    for (idx, part) in parts.into_iter().enumerate() {
        if idx > 0 {
            out.push(b'&');
        }
        out.extend(part);
    }
    out
}

fn append_query(url: &mut String, query: &str) {
    if query.is_empty() {
        return;
    }
    if url.contains('?') {
        url.push('&');
    } else {
        url.push('?');
    }
    url.push_str(query);
}

fn next_value(tokens: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    tokens
        .get(*index)
        .cloned()
        .ok_or_else(|| format!("参数 {} 缺少值", option))
}

fn shell_words(input: &str) -> Result<Vec<String>, String> {
    let text = input.trim_start_matches('\u{feff}').trim();
    let text = text.strip_prefix("$ ").unwrap_or(text);
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            Some('"') => {
                if ch == '"' {
                    quote = None;
                } else if ch == '\\' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else {
                    current.push(ch);
                }
            }
            _ => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => {
                    if matches!(chars.peek(), Some('\n')) {
                        chars.next();
                    } else if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
        }
    }

    if quote.is_some() {
        return Err("cURL 命令存在未闭合的引号".to_string());
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::parse_curl;

    #[test]
    fn parse_browser_bash_curl_post() {
        let parsed = parse_curl(
            "curl 'http://example.com/login' -H 'Content-Type: application/x-www-form-urlencoded' --data-raw 'username=a&password=b'",
        )
        .unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.url, "http://example.com/login");
        assert_eq!(parsed.headers[0].0, "Content-Type");
        assert_eq!(parsed.body.unwrap(), b"username=a&password=b");
    }

    #[test]
    fn parse_multiline_curl() {
        let parsed =
            parse_curl("curl 'http://example.com' \\\n  -H 'A: B' \\\n  --data 'x=1'").unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.headers[0], ("A".to_string(), "B".to_string()));
        assert_eq!(parsed.body.unwrap(), b"x=1");
    }

    #[test]
    fn parse_get_with_query_data() {
        let parsed = parse_curl("curl -G 'http://example.com/search' --data 'q=a'").unwrap();
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.url, "http://example.com/search?q=a");
        assert!(parsed.body.is_none());
    }
}

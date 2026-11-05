use anyhow::{Result, anyhow};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub fn serve(report_dir: &Path, host: &str, port: u16, open: bool) -> Result<()> {
    let bind_address = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let server =
        Server::http(&bind_address).map_err(|error| anyhow!("无法监听 {bind_address}: {error}"))?;

    let display_host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let url = format!("http://{display_host}:{port}/");
    let index_file = report_dir.join("index.html");

    println!("sexlog 仪表盘: {url}");
    println!("静态文件: {}", index_file.display());
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        println!("注意: 当前监听 {host}，同一网络中的其他设备也可以访问，请谨慎使用。");
    }
    if open {
        open_browser(&url);
    }
    println!("按 Ctrl+C 停止");

    for request in server.incoming_requests() {
        if let Err(error) = handle(request, &index_file) {
            eprintln!("请求处理失败: {error}");
        }
    }

    Ok(())
}

fn handle(request: Request, index_file: &Path) -> std::io::Result<()> {
    if request.method() != &Method::Get && request.method() != &Method::Head {
        return respond(
            request,
            Response::from_string("Method Not Allowed").with_status_code(StatusCode(405)),
            Some("text/plain; charset=utf-8"),
        );
    }

    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("/");

    match path {
        "/" | "/index.html" => match fs::read_to_string(index_file) {
            Ok(html) => respond(
                request,
                Response::from_string(html),
                Some("text/html; charset=utf-8"),
            ),
            Err(_) => respond(
                request,
                Response::from_string("仪表盘尚未生成，请运行 sexlog report 后刷新。")
                    .with_status_code(StatusCode(404)),
                Some("text/plain; charset=utf-8"),
            ),
        },
        "/favicon.ico" => respond(request, Response::empty(StatusCode(204)), None),
        _ => respond(
            request,
            Response::from_string("Not Found").with_status_code(StatusCode(404)),
            Some("text/plain; charset=utf-8"),
        ),
    }
}

fn respond<R: Read>(
    request: Request,
    mut response: Response<R>,
    content_type: Option<&str>,
) -> std::io::Result<()> {
    if let Some(content_type) = content_type {
        response.add_header(header("Content-Type", content_type));
    }
    response.add_header(header("Cache-Control", "no-store"));
    response.add_header(header("X-Content-Type-Options", "nosniff"));
    response.add_header(header("Referrer-Policy", "no-referrer"));
    response.add_header(header("Content-Security-Policy", CSP));
    request.respond(response)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static header must be valid")
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let spawned = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let spawned = Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(url)
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = Command::new("xdg-open").arg(url).spawn();
    #[cfg(not(any(unix, windows)))]
    let spawned: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "当前平台不支持自动打开浏览器",
    ));

    if let Err(error) = spawned {
        eprintln!("无法自动打开浏览器: {error}");
    }
}

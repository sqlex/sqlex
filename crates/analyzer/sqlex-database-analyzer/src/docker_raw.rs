use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
};

use serde::Deserialize;

pub const MANAGED_LABEL_KEY: &str = "sqlex.managed";
pub const MANAGED_LABEL_VALUE: &str = "true";
pub const DIALECT_LABEL_KEY: &str = "sqlex.dialect";

#[derive(Deserialize)]
struct DockerContainerSummary {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Labels")]
    labels: Option<HashMap<String, String>>,
}

pub fn cleanup_container_only_sync(container_id: &str) {
    #[cfg(any(unix, windows))]
    {
        if remove_container_via_http(container_id).is_ok() {
            return;
        }
    }

    let _ = std::process::Command::new("docker")
        .args(["rm", "-f", "-v", container_id])
        .output();
}

pub fn list_managed_container_ids() -> Vec<String> {
    list_managed_container_ids_via_http()
        .or_else(|_| list_managed_container_ids_via_cli())
        .unwrap_or_default()
}

fn list_managed_container_ids_via_http() -> std::io::Result<Vec<String>> {
    let response = send_docker_http_request_raw("GET", "/containers/json?all=1")?;
    if parse_http_status(&response)? != 200 {
        return Err(std::io::Error::other("Unexpected Docker list status"));
    }

    let containers: Vec<DockerContainerSummary> =
        serde_json::from_slice(http_response_body(&response)).map_err(|e| {
            std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse Docker container list: {}", e),
            )
        })?;

    Ok(containers
        .into_iter()
        .filter(|container| {
            container
                .labels
                .as_ref()
                .and_then(|labels| labels.get(MANAGED_LABEL_KEY))
                .map(String::as_str)
                == Some(MANAGED_LABEL_VALUE)
        })
        .map(|container| container.id)
        .collect())
}

fn list_managed_container_ids_via_cli() -> std::io::Result<Vec<String>> {
    let label_filter = format!("label={}={}", MANAGED_LABEL_KEY, MANAGED_LABEL_VALUE);
    let output = std::process::Command::new("docker")
        .args(["ps", "-aq", "--filter", &label_filter])
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(
            "Failed to list managed containers via docker CLI",
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

#[cfg(any(unix, windows))]
fn remove_container_via_http(container_id: &str) -> std::io::Result<()> {
    let stop_path = format!("/containers/{}/stop?t=5", container_id);
    let _ = send_docker_http_request_raw("POST", &stop_path);

    let remove_path = format!("/containers/{}?force=1&v=1", container_id);
    let status = parse_http_status(&send_docker_http_request_raw("DELETE", &remove_path)?)?;
    if matches!(status, 200 | 204 | 404) {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "Unexpected Docker delete status: {}",
            status
        )))
    }
}

#[cfg(not(any(unix, windows)))]
fn remove_container_via_http(_container_id: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        ErrorKind::Unsupported,
        "Raw Docker HTTP is only supported on unix/windows",
    ))
}

#[cfg(unix)]
fn send_docker_http_request_raw(method: &str, path: &str) -> std::io::Result<Vec<u8>> {
    use std::os::unix::net::UnixStream;

    let socket_path = std::env::var("DOCKER_HOST")
        .ok()
        .and_then(|host| host.strip_prefix("unix://").map(str::to_owned))
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "/var/run/docker.sock".to_string());

    let mut stream = UnixStream::connect(socket_path)?;
    send_http_request(&mut stream, method, path)
}

#[cfg(windows)]
fn send_docker_http_request_raw(method: &str, path: &str) -> std::io::Result<Vec<u8>> {
    let pipe_path = std::env::var("DOCKER_HOST")
        .ok()
        .and_then(|host| {
            host.strip_prefix("npipe://")
                .map(|pipe| pipe.replace('/', "\\"))
        })
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| r"\\.\pipe\docker_engine".to_string());

    let mut pipe = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe_path)?;
    send_http_request(&mut pipe, method, path)
}

#[cfg(not(any(unix, windows)))]
fn send_docker_http_request_raw(_method: &str, _path: &str) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::new(
        ErrorKind::Unsupported,
        "Raw Docker HTTP is only supported on unix/windows",
    ))
}

fn parse_http_status(response: &[u8]) -> std::io::Result<u16> {
    String::from_utf8_lossy(response)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(ErrorKind::InvalidData, "Failed to parse Docker HTTP status")
        })
}

fn http_response_body(response: &[u8]) -> &[u8] {
    response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map_or(response, |pos| &response[pos + 4..])
}

fn send_http_request<RW>(stream: &mut RW, method: &str, path: &str) -> std::io::Result<Vec<u8>>
where
    RW: Read + Write,
{
    let request = format!(
        "{} {} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        method, path
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    Ok(response)
}

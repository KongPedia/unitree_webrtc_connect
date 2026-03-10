use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;
use std::collections::HashMap;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};

const RECV_PORT: u16 = 10134;
const MULTICAST_GROUP: &str = "231.1.1.1";
const MULTICAST_PORT: u16 = 10131;

pub fn discover_ip_sn(timeout_secs: f64) -> Result<HashMap<String, String>, String> {
    let mut serial_to_ip: HashMap<String, String> = HashMap::new();

    let socket = UdpSocket::bind(("0.0.0.0", RECV_PORT))
        .map_err(|e| format!("Failed to bind multicast socket: {e}"))?;

    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;

    let group: Ipv4Addr = MULTICAST_GROUP
        .parse()
        .map_err(|e| format!("Invalid multicast group: {e}"))?;

    socket
        .join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED)
        .map_err(|e| format!("Failed to join multicast group: {e}"))?;

    let query_message = r#"{"name":"unitree_dapengche"}"#;
    socket
        .send_to(query_message.as_bytes(), (MULTICAST_GROUP, MULTICAST_PORT))
        .map_err(|e| format!("Failed to send multicast query: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs.max(0.0));
    let mut buffer = [0u8; 1024];

    while Instant::now() < deadline {
        match socket.recv_from(&mut buffer) {
            Ok((size, addr)) => {
                let payload = match std::str::from_utf8(&buffer[..size]) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let parsed: Value = match serde_json::from_str(payload) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let Some(sn) = parsed.get("sn").and_then(Value::as_str) else {
                    continue;
                };

                let ip = match parsed.get("ip").and_then(Value::as_str) {
                    Some(v) => v.to_string(),
                    None => addr.ip().to_string(),
                };

                serial_to_ip.insert(sn.to_string(), ip);
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => return Err(format!("Multicast receive error: {e}")),
        }
    }

    Ok(serial_to_ip)
}

#[pyfunction]
#[pyo3(signature = (timeout=2.0))]
pub fn discover_ip_sn_py(py: Python<'_>, timeout: f64) -> PyResult<PyObject> {
    let map = discover_ip_sn(timeout).map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
    let dict = PyDict::new_bound(py);

    for (sn, ip) in map {
        dict.set_item(sn, ip)?;
    }

    Ok(dict.into_py(py))
}

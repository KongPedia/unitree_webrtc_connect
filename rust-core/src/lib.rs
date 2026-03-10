#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;
use pyo3::PyErr;
use serde_json::Value;
use std::sync::Mutex;

pub mod audio;
pub mod audiohub;
pub mod connection;
pub mod constants;
pub mod datachannel;
pub mod encryption;
pub mod lidar;
pub mod message;
pub mod multicast;
pub mod msgs;
pub mod signaling;
pub mod utils;
pub mod video;

#[pyclass(name = "UnitreeWebRTCConnection")]
struct PyUnitreeWebRTCConnection {
    inner: Mutex<connection::UnitreeWebRTCConnection>,
}

#[pymethods]
#[allow(clippy::useless_conversion)]
impl PyUnitreeWebRTCConnection {
    #[new]
    #[pyo3(signature = (connection_method, serial_number=None, ip=None, username=None, password=None))]
    fn new(
        connection_method: constants::WebRTCConnectionMethod,
        serial_number: Option<String>,
        ip: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let _ = (username, password);
        Self {
            inner: Mutex::new(connection::UnitreeWebRTCConnection::new(
                connection_method,
                serial_number,
                ip,
            )),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn connect(&self) -> PyResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Runtime init failed: {e}"
                )))
            }
        };
        match rt.block_on(inner.connect()) {
            Ok(()) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Connect failed: {e}"
            ))),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn disconnect(&self) -> PyResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Runtime init failed: {e}"
                )))
            }
        };
        rt.block_on(inner.disconnect());
        Ok(())
    }

    #[allow(clippy::useless_conversion)]
    fn reconnect(&self) -> PyResult<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Runtime init failed: {e}"
                )))
            }
        };
        match rt.block_on(inner.reconnect()) {
            Ok(()) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Reconnect failed: {e}"
            ))),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn auto_reconnect(&self, max_retries: u32) -> PyResult<bool> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Runtime init failed: {e}"
                )))
            }
        };
        Ok(rt.block_on(inner.auto_reconnect(max_retries)))
    }

    #[allow(clippy::useless_conversion)]
    fn is_connected(&self) -> PyResult<bool> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        Ok(inner.is_connected)
    }

    #[allow(clippy::useless_conversion)]
    fn ip(&self) -> PyResult<Option<String>> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };
        Ok(inner.ip.clone())
    }

    #[allow(clippy::useless_conversion)]
    fn publish_request_new(&self, topic: &str, payload_json: &str) -> PyResult<String> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Lock poisoned",
                ))
            }
        };

        let payload: Value = serde_json::from_str(payload_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid payload JSON: {e}"))
        })?;

        let response = inner.publish_request_new(topic, payload).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Publish failed: {e}"))
        })?;

        serde_json::to_string(&response).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Response serialization failed: {e}"
            ))
        })
    }

    fn rust_bridge_ping(&self) -> String {
        "pong".to_string()
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn unitree_webrtc_core_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Create a submodule for `constants`
    let constants_module = PyModule::new_bound(_py, "constants")?;
    constants::register_constants(_py, &constants_module)?;
    m.add_submodule(&constants_module)?;
    m.add_class::<PyUnitreeWebRTCConnection>()?;
    m.add_class::<constants::WebRTCConnectionMethod>()?;
    m.add_function(wrap_pyfunction!(multicast::discover_ip_sn_py, m)?)?;

    lidar::register(m)?;

    // Register in sys.modules so `from unitree_webrtc_core_rs.constants import ...` works
    let sys = _py.import_bound("sys")?;
    let sys_modules = sys.getattr("modules")?;
    sys_modules.set_item("unitree_webrtc_core_rs.constants", &constants_module)?;

    Ok(())
}

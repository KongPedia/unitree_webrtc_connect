#![allow(clippy::useless_conversion)]
#![allow(clippy::await_holding_lock)]

use pyo3::prelude::*;
use pyo3::PyErr;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub mod audio;
pub mod audiohub;
pub mod callback_bridge;
pub mod connection;
pub mod constants;
pub mod datachannel;
pub mod encryption;
pub mod lidar;
pub mod message;
pub mod msgs;
pub mod multicast;
pub mod signaling;
pub mod utils;
pub mod video;

#[pyclass(name = "UnitreeWebRTCConnection")]
struct PyUnitreeWebRTCConnection {
    inner: Arc<Mutex<connection::UnitreeWebRTCConnection>>,
    rt: Arc<tokio::runtime::Runtime>,
    callback_bridge: Arc<callback_bridge::CallbackBridge>,
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
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio Runtime");
        Self {
            inner: Arc::new(Mutex::new(connection::UnitreeWebRTCConnection::new(
                connection_method,
                serial_number,
                ip,
            ))),
            rt: Arc::new(rt),
            callback_bridge: Arc::new(callback_bridge::CallbackBridge::new()),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn connect(&self) -> PyResult<()> {
        let inner_clone = Arc::clone(&self.inner);
        let connect_future = async move {
            let mut inner = inner_clone.lock().unwrap();
            inner.connect(inner_clone.clone()).await
        };

        match self.rt.block_on(connect_future) {
            Ok(()) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Connect failed: {e}"
            ))),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn disconnect(&self) -> PyResult<()> {
        let inner_clone = Arc::clone(&self.inner);
        let disconnect_future = async move {
            let mut inner = inner_clone.lock().unwrap();
            inner.disconnect().await;
        };
        self.rt.block_on(disconnect_future);
        Ok(())
    }

    #[allow(clippy::useless_conversion)]
    fn reconnect(&self) -> PyResult<()> {
        let inner_clone = Arc::clone(&self.inner);
        let reconnect_future = async move {
            let mut inner = inner_clone.lock().unwrap();
            inner.reconnect(inner_clone.clone()).await
        };
        match self.rt.block_on(reconnect_future) {
            Ok(()) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Reconnect failed: {e}"
            ))),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn auto_reconnect(&self, max_retries: u32) -> PyResult<bool> {
        let inner_clone = Arc::clone(&self.inner);
        let auto_reconnect_future = async move {
            let mut inner = inner_clone.lock().unwrap();
            inner.auto_reconnect(inner_clone.clone(), max_retries).await
        };
        Ok(self.rt.block_on(auto_reconnect_future))
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
    fn wait_datachannel_open(&self, timeout: f64) -> PyResult<()> {
        let inner_clone = Arc::clone(&self.inner);
        let wait_future = async move {
            let start = std::time::Instant::now();
            let dur = std::time::Duration::from_secs_f64(timeout);
            loop {
                let is_open = {
                    let inner = inner_clone.lock().unwrap();
                    inner
                        .datachannel
                        .as_ref()
                        .map(|dc| dc.is_open())
                        .unwrap_or(false)
                };

                if is_open {
                    return Ok(());
                }

                if start.elapsed() > dur {
                    return Err("Data channel did not open in time".to_string());
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        };

        match self.rt.block_on(wait_future) {
            Ok(()) => Ok(()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyTimeoutError, _>(e)),
        }
    }

    #[allow(clippy::useless_conversion)]
    fn publish_request_new(&self, topic: &str, payload_json: &str) -> PyResult<String> {
        let payload: Value = serde_json::from_str(payload_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid payload JSON: {e}"))
        })?;

        let inner_clone = Arc::clone(&self.inner);
        let topic_str = topic.to_string();

        let req_future = async move {
            let key = {
                let mut inner = inner_clone.lock().unwrap();
                inner.publish_send_req(&topic_str, payload)?
            };

            let start = std::time::Instant::now();
            let dur = std::time::Duration::from_secs(5);
            loop {
                {
                    let mut inner = inner_clone.lock().unwrap();
                    if let Some(dc) = &mut inner.datachannel {
                        if let Some(resolved) = dc.take_resolved(&key) {
                            return Ok(resolved);
                        }
                    }
                }
                if start.elapsed() > dur {
                    return Err("Timeout waiting for response".to_string());
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        };

        let response = self.rt.block_on(req_future).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Publish failed: {e}"))
        })?;

        serde_json::to_string(&response).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Response serialization failed: {e}"
            ))
        })
    }

    #[allow(clippy::useless_conversion)]
    #[pyo3(signature = (topic, data_json, msg_type))]
    fn publish_without_callback(
        &self,
        topic: &str,
        data_json: &str,
        msg_type: &str,
    ) -> PyResult<()> {
        let payload: Value = serde_json::from_str(data_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid payload JSON: {e}"))
        })?;

        let mut inner = self.inner.lock().unwrap();
        inner
            .publish_without_callback(topic, Some(payload), msg_type)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Publish failed: {e}"))
            })
    }

    #[allow(clippy::useless_conversion)]
    #[pyo3(signature = (topic, cb))]
    fn subscribe(&self, topic: &str, cb: PyObject) -> PyResult<()> {
        let callback = self.callback_bridge.create_callback(cb);
        let mut inner = self.inner.lock().unwrap();
        inner.subscribe(topic, callback).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Subscribe failed: {e}"))
        })
    }

    #[allow(clippy::useless_conversion)]
    fn unsubscribe(&self, topic: &str) -> PyResult<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.unsubscribe(topic).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Unsubscribe failed: {e}"))
        })
    }

    #[allow(clippy::useless_conversion)]
    fn disable_traffic_saving(&self, switch: bool) -> PyResult<bool> {
        let mut inner = self.inner.lock().unwrap();
        let message = serde_json::json!({
            "req_type": "disable_traffic_saving",
            "instruction": if switch { "on" } else { "off" }
        });

        inner
            .publish_without_callback("", Some(message), "rtc_inner_req")
            .map(|_| true)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Error: {e}")))
    }

    #[allow(clippy::useless_conversion)]
    fn switch_video_channel(&self, switch: bool) -> PyResult<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .publish_without_callback(
                "",
                Some(serde_json::Value::String(if switch {
                    "on".to_string()
                } else {
                    "off".to_string()
                })),
                "vid",
            )
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Error: {e}")))
    }

    #[allow(clippy::useless_conversion)]
    fn switch_audio_channel(&self, switch: bool) -> PyResult<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .publish_without_callback(
                "",
                Some(serde_json::Value::String(if switch {
                    "on".to_string()
                } else {
                    "off".to_string()
                })),
                "aud",
            )
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Error: {e}")))
    }

    #[allow(clippy::useless_conversion)]
    fn set_decoder(&self, _decoder_type: &str) -> PyResult<()> {
        Ok(())
    }

    #[allow(clippy::useless_conversion)]
    #[pyo3(signature = (cb))]
    fn add_video_track_callback(&self, cb: pyo3::PyObject) -> PyResult<()> {
        let callback = self.callback_bridge.create_json_callback(cb);
        let mut inner = self.inner.lock().unwrap();
        if let Some(video) = &mut inner.video {
            video.add_track_callback(callback);
        }
        Ok(())
    }

    #[allow(clippy::useless_conversion)]
    #[pyo3(signature = (cb))]
    fn add_audio_track_callback(&self, cb: pyo3::PyObject) -> PyResult<()> {
        let callback = self.callback_bridge.create_json_callback(cb);
        let mut inner = self.inner.lock().unwrap();
        if let Some(audio) = &mut inner.audio {
            audio.add_track_callback(callback);
        }
        Ok(())
    }

    fn rust_bridge_ping(&self) -> String {
        "pong".to_string()
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn unitree_webrtc_connect_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Create a submodule for `constants`
    let constants_module = PyModule::new_bound(_py, "constants")?;
    constants::register_constants(_py, &constants_module)?;
    m.add_submodule(&constants_module)?;
    m.add_class::<PyUnitreeWebRTCConnection>()?;
    m.add_class::<constants::WebRTCConnectionMethod>()?;
    m.add_function(wrap_pyfunction!(multicast::discover_ip_sn_py, m)?)?;

    lidar::register(m)?;

    // Register in sys.modules so `from unitree_webrtc_connect_rs.constants import ...` works
    let sys = _py.import_bound("sys")?;
    let sys_modules = sys.getattr("modules")?;
    sys_modules.set_item("unitree_webrtc_connect_rs.constants", &constants_module)?;

    Ok(())
}

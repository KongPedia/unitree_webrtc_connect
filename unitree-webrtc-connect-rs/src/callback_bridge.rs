use crate::msgs::pubsub::LidarDecodedData;
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct CallbackBridge {
    tx: mpsc::UnboundedSender<(Arc<PyObject>, String, Option<LidarDecodedData>)>,
}

impl Default for CallbackBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackBridge {
    pub fn new() -> Self {
        let (tx, mut rx) =
            mpsc::unbounded_channel::<(Arc<PyObject>, String, Option<LidarDecodedData>)>();

        // Spawn a dedicated blocking thread to handle Python callbacks safely
        // This avoids acquiring the GIL on the async worker threads!
        std::thread::spawn(move || {
            while let Some((cb_arc, json_msg, decoded_opt)) = rx.blocking_recv() {
                Python::with_gil(|py| {
                    let py_json = match py.import_bound("json") {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("CallbackBridge failed to import json: {}", e);
                            return;
                        }
                    };

                    if let Ok(loads) = py_json.getattr("loads") {
                        if let Ok(py_dict) = loads.call1((json_msg,)) {
                            if let Some(decoded) = decoded_opt {
                                if let Ok(py_dict_obj) = py_dict.downcast::<pyo3::types::PyDict>() {
                                    if let Ok(Some(data_obj)) = py_dict_obj.get_item("data") {
                                        if let Ok(data_dict) =
                                            data_obj.downcast::<pyo3::types::PyDict>()
                                        {
                                            match decoded {
                                                LidarDecodedData::Native(points) => {
                                                    let n_points = points.len();
                                                    let mut flat = Vec::with_capacity(n_points * 3);
                                                    for p in points {
                                                        flat.push(p[0]);
                                                        flat.push(p[1]);
                                                        flat.push(p[2]);
                                                    }
                                                    if let Ok(array) =
                                                        ndarray::Array2::from_shape_vec(
                                                            (n_points, 3),
                                                            flat,
                                                        )
                                                    {
                                                        let py_array =
                                                            numpy::IntoPyArray::into_pyarray_bound(
                                                                array, py,
                                                            );
                                                        let result_dict =
                                                            pyo3::types::PyDict::new_bound(py);
                                                        let _ = result_dict
                                                            .set_item("points", py_array);
                                                        let _ =
                                                            data_dict.set_item("data", result_dict);
                                                    }
                                                }
                                                LidarDecodedData::Wasm {
                                                    point_count,
                                                    face_count,
                                                    positions,
                                                    uvs,
                                                    indices,
                                                } => {
                                                    let result_dict =
                                                        pyo3::types::PyDict::new_bound(py);
                                                    let _ = result_dict
                                                        .set_item("point_count", point_count);
                                                    let _ = result_dict
                                                        .set_item("face_count", face_count);

                                                    let p_arr = numpy::PyArray1::from_vec_bound(
                                                        py, positions,
                                                    );
                                                    let u_arr =
                                                        numpy::PyArray1::from_vec_bound(py, uvs);
                                                    let o_arr = numpy::PyArray1::from_vec_bound(
                                                        py, indices,
                                                    );

                                                    let _ =
                                                        result_dict.set_item("positions", p_arr);
                                                    let _ = result_dict.set_item("uvs", u_arr);
                                                    let _ = result_dict.set_item("indices", o_arr);

                                                    let _ = data_dict.set_item("data", result_dict);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Err(e) = cb_arc.call1(py, (py_dict,)) {
                                eprintln!("Callback execution failed: {}", e);
                            }
                        }
                    }
                });
            }
        });

        Self { tx }
    }

    pub fn create_callback(&self, py_cb: PyObject) -> crate::msgs::pubsub::TopicCallback {
        let tx = self.tx.clone();
        let py_cb_arc = Arc::new(py_cb);
        Box::new(
            move |payload: crate::msgs::pubsub::CallbackPayload| match payload {
                crate::msgs::pubsub::CallbackPayload::Json(value) => {
                    if let Ok(json_str) = serde_json::to_string(&value) {
                        let _ = tx.send((Arc::clone(&py_cb_arc), json_str, None));
                    }
                }
                crate::msgs::pubsub::CallbackPayload::Lidar(value, decoded_data) => {
                    if let Ok(json_str) = serde_json::to_string(&value) {
                        let _ = tx.send((Arc::clone(&py_cb_arc), json_str, Some(decoded_data)));
                    }
                }
            },
        )
    }

    pub fn create_json_callback(
        &self,
        py_cb: PyObject,
    ) -> Box<dyn Fn(&serde_json::Value) + Send + Sync + 'static> {
        let tx = self.tx.clone();
        let py_cb_arc = Arc::new(py_cb);
        Box::new(move |value: &serde_json::Value| {
            if let Ok(json_str) = serde_json::to_string(value) {
                let _ = tx.send((Arc::clone(&py_cb_arc), json_str, None));
            }
        })
    }
}

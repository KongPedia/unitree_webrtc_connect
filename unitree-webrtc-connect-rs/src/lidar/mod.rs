#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;
use pyo3::types::PyDict;

pub mod native;
pub mod wasm;

use native::NativeLidarDecoder;
use wasm::WasmLidarDecoder;

#[pyclass]
pub struct UnifiedLidarDecoder {
    decoder_type: String,
    native: NativeLidarDecoder,
    wasm: WasmLidarDecoder,
}

#[pymethods]
impl UnifiedLidarDecoder {
    #[new]
    pub fn new() -> PyResult<Self> {
        Ok(UnifiedLidarDecoder {
            decoder_type: "native".to_string(),
            native: NativeLidarDecoder::new(),
            wasm: WasmLidarDecoder::new()?,
        })
    }

    pub fn set_decoder(&mut self, decoder: String) -> PyResult<()> {
        if decoder != "native" && decoder != "libvoxel" {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid decoder type. Must be 'native' or 'libvoxel'.",
            ));
        }
        self.decoder_type = decoder;
        Ok(())
    }

    pub fn decode<'py>(
        &self,
        py: Python<'py>,
        compressed_data: &[u8],
        origin_list: Vec<f64>,
        resolution: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        if origin_list.len() != 3 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Origin must be a list of 3 floats",
            ));
        }
        let origin = [origin_list[0], origin_list[1], origin_list[2]];

        if self.decoder_type == "native" {
            self.native
                .decode(py, compressed_data, 1000000, origin, resolution)
        } else {
            self.wasm.decode(py, compressed_data, origin, resolution)
        }
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<native::NativeLidarDecoder>()?;
    m.add_class::<wasm::WasmLidarDecoder>()?;
    m.add_class::<UnifiedLidarDecoder>()?;
    Ok(())
}

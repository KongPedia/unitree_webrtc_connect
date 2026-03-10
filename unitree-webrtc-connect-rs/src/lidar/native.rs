#![allow(clippy::useless_conversion)]

use numpy::IntoPyArray;
use pyo3::prelude::*;

pub fn decode_native_core(
    compressed_data: &[u8],
    src_size: usize,
    origin: [f64; 3],
    resolution: f64,
) -> Result<Vec<[f64; 3]>, String> {
    let decompressed = lz4_flex::block::decompress(compressed_data, src_size)
        .map_err(|e| format!("LZ4 decomp failed: {}", e))?;

    let mut points: Vec<[f64; 3]> = Vec::with_capacity(1000);

    for (i, &byte) in decompressed.iter().enumerate() {
        if byte == 0 {
            continue;
        }

        let z = i / 0x800;
        let n_slice = i % 0x800;
        let y = n_slice / 0x10;
        let x_base = (n_slice % 0x10) * 8;

        for bit in 0..8 {
            if (byte & (1 << (7 - bit))) != 0 {
                let x = x_base + bit;
                let final_x = (x as f64) * resolution + origin[0];
                let final_y = (y as f64) * resolution + origin[1];
                let final_z = (z as f64) * resolution + origin[2];
                points.push([final_x, final_y, final_z]);
            }
        }
    }

    Ok(points)
}

/// Decodes lidar data matching the `lidar_decoder_native.py` logic.
#[pyclass]
#[derive(Default)]
pub struct NativeLidarDecoder;

#[pymethods]
impl NativeLidarDecoder {
    #[new]
    pub fn new() -> Self {
        NativeLidarDecoder
    }

    /// Decode compressed data with given origin and resolution
    pub fn decode<'py>(
        &self,
        py: Python<'py>,
        compressed_data: &[u8],
        src_size: usize,
        origin: [f64; 3],
        resolution: f64,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let points = decode_native_core(compressed_data, src_size, origin, resolution)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;

        let n_points = points.len();
        let mut flat = Vec::with_capacity(n_points * 3);
        for p in points {
            flat.push(p[0]);
            flat.push(p[1]);
            flat.push(p[2]);
        }

        let array = ndarray::Array2::from_shape_vec((n_points, 3), flat)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let py_array = array.into_pyarray_bound(py);

        let result_dict = pyo3::types::PyDict::new_bound(py);
        result_dict.set_item("points", py_array)?;

        Ok(result_dict)
    }
}

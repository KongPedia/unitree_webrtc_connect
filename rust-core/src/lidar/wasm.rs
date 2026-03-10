#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::OnceLock;
use wasmtime::*;

static ENGINE: OnceLock<Engine> = OnceLock::new();
static MODULE: OnceLock<Module> = OnceLock::new();

type WasmDecodeOutput = (usize, usize, Vec<f32>, Vec<f32>, Vec<u32>);

#[pyclass]
pub struct WasmLidarDecoder;

pub fn decode_wasm_core(
    compressed_data: &[u8],
    origin: [f64; 3],
    resolution: f64,
) -> Result<WasmDecodeOutput, String> {
    let engine = ENGINE.get().unwrap();
    let module = MODULE.get().unwrap();
    let mut store = Store::new(engine, ());

    let mut linker = Linker::new(engine);

    linker
        .func_wrap("env", "a", |mut caller: Caller<'_, ()>, _t: i32| -> i32 {
            let mem = caller.get_export("c").unwrap().into_memory().unwrap();
            mem.data_size(&mut caller) as i32
        })
        .map_err(|e| e.to_string())?;

    linker
        .func_wrap(
            "env",
            "b",
            |mut caller: Caller<'_, ()>, target: i32, start: i32, len: i32| {
                let mem = caller.get_export("c").unwrap().into_memory().unwrap();
                let data = mem.data_mut(&mut caller);
                let t = target as usize;
                let s = start as usize;
                let l = len as usize;
                if t + l <= data.len() && s + l <= data.len() {
                    data.copy_within(s..s + l, t);
                }
            },
        )
        .map_err(|e| e.to_string())?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| e.to_string())?;

    let generate = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32), ()>(
            &mut store, "e",
        )
        .map_err(|e| e.to_string())?;
    let malloc = instance
        .get_typed_func::<i32, i32>(&mut store, "f")
        .map_err(|e| e.to_string())?;
    let memory = instance
        .get_memory(&mut store, "c")
        .ok_or_else(|| "WASM missing module memory 'c'".to_string())?;

    let input_ptr = malloc.call(&mut store, 61440).map_err(|e| e.to_string())?;
    let decompress_buffer_size = 80000;
    let decompress_buffer_ptr = malloc.call(&mut store, decompress_buffer_size).unwrap();
    let positions_ptr = malloc.call(&mut store, 2880000).unwrap();
    let uvs_ptr = malloc.call(&mut store, 1920000).unwrap();
    let indices_ptr = malloc.call(&mut store, 5760000).unwrap();
    let decompressed_size_ptr = malloc.call(&mut store, 4).unwrap();
    let face_count_ptr = malloc.call(&mut store, 4).unwrap();
    let point_count_ptr = malloc.call(&mut store, 4).unwrap();

    let input_slice = &mut memory.data_mut(&mut store)
        [input_ptr as usize..input_ptr as usize + compressed_data.len()];
    input_slice.copy_from_slice(compressed_data);

    let some_v = (origin[2] / resolution).floor() as i32;

    generate
        .call(
            &mut store,
            (
                input_ptr,
                compressed_data.len() as i32,
                decompress_buffer_size,
                decompress_buffer_ptr,
                decompressed_size_ptr,
                positions_ptr,
                uvs_ptr,
                indices_ptr,
                face_count_ptr,
                point_count_ptr,
                some_v,
            ),
        )
        .map_err(|e| e.to_string())?;

    let mem_data = memory.data(&store);

    let c_bytes = &mem_data[point_count_ptr as usize..point_count_ptr as usize + 4];
    let point_count = i32::from_le_bytes(c_bytes.try_into().unwrap()) as usize;

    let u_bytes = &mem_data[face_count_ptr as usize..face_count_ptr as usize + 4];
    let face_count = i32::from_le_bytes(u_bytes.try_into().unwrap()) as usize;

    let positions_bytes =
        &mem_data[positions_ptr as usize..positions_ptr as usize + face_count * 12];
    let uvs_bytes = &mem_data[uvs_ptr as usize..uvs_ptr as usize + face_count * 8];
    let indices_bytes = &mem_data[indices_ptr as usize..indices_ptr as usize + face_count * 24];

    // Convert to Vecs
    let mut positions = Vec::with_capacity(positions_bytes.len() / 4);
    for chunk in positions_bytes.chunks_exact(4) {
        positions.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }

    let mut uvs = Vec::with_capacity(uvs_bytes.len() / 4);
    for chunk in uvs_bytes.chunks_exact(4) {
        uvs.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }

    let mut indices = Vec::with_capacity(face_count * 6);
    for chunk in indices_bytes.chunks_exact(4) {
        indices.push(u32::from_le_bytes(chunk.try_into().unwrap()));
    }

    Ok((point_count, face_count, positions, uvs, indices))
}

#[pymethods]
impl WasmLidarDecoder {
    #[new]
    pub fn new() -> PyResult<Self> {
        let _ = WasmLidarDecoder::get_module();
        Ok(WasmLidarDecoder)
    }

    pub fn decode<'py>(
        &self,
        py: Python<'py>,
        compressed_data: &[u8],
        origin: [f64; 3],
        resolution: f64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let (point_count, face_count, positions, uvs, indices) =
            decode_wasm_core(compressed_data, origin, resolution)
                .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

        let result = PyDict::new_bound(py);
        result.set_item("point_count", point_count)?;
        result.set_item("face_count", face_count)?;

        let p_arr = numpy::PyArray1::from_vec_bound(py, positions);
        let u_arr = numpy::PyArray1::from_vec_bound(py, uvs);
        let o_arr = numpy::PyArray1::from_vec_bound(py, indices);

        result.set_item("positions", p_arr)?;
        result.set_item("uvs", u_arr)?;
        result.set_item("indices", o_arr)?;

        Ok(result)
    }
}

impl WasmLidarDecoder {
    fn get_module() -> &'static Module {
        MODULE.get_or_init(|| {
            let mut config = Config::new();
            config.wasm_multi_value(true);
            let engine = ENGINE.get_or_init(|| Engine::new(&config).unwrap());
            let wasm_bytes = include_bytes!("libvoxel.wasm");
            Module::new(engine, wasm_bytes).unwrap()
        })
    }
}

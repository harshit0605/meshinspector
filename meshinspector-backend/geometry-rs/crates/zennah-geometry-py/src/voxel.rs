use pyo3::prelude::*;

mod conversion;
mod marching;
mod ops;
mod raw;
mod rendering;
mod sampling;
mod segmentation;

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    raw::register(module)?;
    ops::register(module)?;
    conversion::register(module)?;
    sampling::register(module)?;
    rendering::register(module)?;
    segmentation::register(module)?;
    marching::register(module)?;
    Ok(())
}

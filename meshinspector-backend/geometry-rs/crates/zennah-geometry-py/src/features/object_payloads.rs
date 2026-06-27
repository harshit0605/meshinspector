use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn feature_object_descriptor_to_py(
    py: Python<'_>,
    descriptor: &zennah_geometry_core::FeatureObjectDescriptor,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("feature_id", &descriptor.feature_id)?;
    output.set_item("source_kind", descriptor.source_kind.as_str())?;
    output.set_item("object_type", descriptor.object_type)?;
    output.set_item("class_name", descriptor.class_name)?;
    output.set_item("class_name_plural", descriptor.class_name_plural)?;
    let properties = PyList::empty(py);
    for property in &descriptor.shared_properties {
        properties.append(feature_object_property_to_py(py, property)?)?;
    }
    output.set_item("shared_properties", properties)?;
    output.set_item("meshlib_reference", descriptor.meshlib_reference)?;
    Ok(output.unbind())
}

fn feature_object_property_to_py(
    py: Python<'_>,
    property: &zennah_geometry_core::FeatureObjectProperty,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("name", property.name)?;
    output.set_item("kind", property.kind.as_str())?;
    output.set_item("scalar_value", property.scalar_value)?;
    output.set_item(
        "vector_value",
        property.vector_value.map(|value| value.to_vec()),
    )?;
    Ok(output.unbind())
}

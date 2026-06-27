use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use numpy::PyReadonlyArray3;
use pyo3::types::{PyAny, PyBytes};
use std::collections::HashMap;
use zennah_geometry_core::{
    MeshlibObjectMeshSceneInput, MeshlibSceneExportInput, MeshlibSceneExportObject,
    MeshlibSceneFeatureObject, MeshlibSceneGroupObject, MeshlibSceneObjectDistanceMap,
    MeshlibSceneObjectLines, MeshlibSceneObjectPoints, MeshlibSceneObjectStateInput,
    MeshlibSceneObjectVoxels, MeshlibSceneFeatureVisualizeProperty,
    MeshlibSceneFeatureVisualizePropertyInput, MeshlibSceneFeatureRenderDimension,
    MeshlibSceneFeatureRenderInput, MeshlibSceneFeatureRenderObject,
    MeshlibSceneFeatureRenderPolyline,
    MeshlibSceneReorderInput, MeshlibSceneReparentInput, MeshlibSceneRibbonAction,
    MeshlibSceneSelectionInput, MeshlibSceneSelectionMode, MeshlibSceneTextureImage,
    MeshlibSceneTransformInput, MeshlibSceneTreeGroupInput, MeshlibSceneTreeRenameInput,
    MeshlibSceneTreeRibbonActionInput, MeshlibSceneTreeUngroupInput, MeshlibSceneChildOrder,
    MeshlibSceneXf, VIEWPORT_MASK_ALL,
};

include!("scene/api_export.rs");
include!("scene/api_edit.rs");
include!("scene/feature_render.rs");
include!("scene/api_ribbon.rs");
include!("scene/api_tree_results.rs");
include!("scene/api_import.rs");
include!("scene/input.rs");
include!("scene/read_objects.rs");
include!("scene/read_child_order.rs");
include!("scene/write_objects.rs");
include!("scene/required.rs");
include!("scene/optional_collections.rs");
include!("scene/texture_options.rs");

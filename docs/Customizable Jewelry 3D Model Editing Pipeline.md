Customizable Jewelry 3D Model Editing Pipeline
To meet the requirement of arbitrary ring sizes and weight-limited models, we can apply mesh offsetting/shelling techniques. Modern geometry libraries allow us to create an inner “shell” inside a model by offsetting surfaces. For example, MeshLib’s Python bindings (mrmeshpy) use a voxel-based OpenVDB approach to build an offset surface at a given distance
. In “Shell Mode,” MeshLib generates two parallel surfaces and discards the original, yielding a hollowed part
. In practice one would load the generated ring’s mesh and call mrmeshpy.offsetMesh(mesh, offset, params) with params.type = Shell to carve out an inner volume (for example, using a negative offset to hollow the interior)
. This effectively reproduces Rhino’s “shell” command: you pass in the 3D mesh and a wall thickness, and it produces a thin-walled hollow object. (Alternatively, a library like pymadcad offers a thicken(surface, distance) function to do exactly this, creating an envelope around the original mesh
.) Blender’s Solidify modifier is another way to achieve a similar result, but relying on a full Blender install can be heavy; using a dedicated geometry library like MeshLib or PyMesh is more lightweight and scriptable for a web service
. The key point is that making the model hollow reduces weight while preserving the external shape. After hollowing, we can compute the resulting volume and thus estimate the mass (weight) by multiplying by material density. Libraries like Trimesh support volume calculation: its mesh.volume property returns the mesh’s volume via a surface integral
. If the hollowed model is still heavier than desired, we simply increase the shell thickness (i.e. deeper offset) or make additional cuts, iterating until the volume (and thus weight) meets the target.
Ring Sizing and Scaling
Jewelry rings come in standard sizes, each corresponding to a specific inner diameter. For example, a US size 6 ring has an inner diameter of about 16.5 mm, size 7 ≈ 17.3 mm, size 8 ≈ 18.2 mm
. To generate all ring sizes from one base model, we first scale the model so that its inner radius matches the target. This can be done by measuring the current model’s inner diameter (e.g. via the mesh’s bounding box or fitted circle) and applying a uniform scale factor. In Python, Trimesh or even Three.js math can apply a scaling matrix to the vertex coordinates. For example, if the current diameter is 16 mm but we need size 8 (18.2 mm), we scale the model by 18.2/16.0 in the ring’s plane. Once scaled for the correct finger size, the ring’s volume will change. We then compute the new volume with Trimesh (or MeshLib) and multiply by the material density to get weight. If the ring is over-weight, we perform the hollowing step described above to reduce material. If it is under-weight (unlikely), we could thicken the walls (a positive offset) until reaching the minimum weight. In any case, this process ensures we can hit a precise weight range. After adjustment, we can (optionally) re-scale the final model slightly to correct any small sizing error introduced by the offsetting process.
Implementation Workflow
A practical implementation would follow these steps:
Prepare Base Model: Load the image-generated GLTF/GLB model of the jewelry (the 3D mesh). Ensure it is watertight (closed surface) so offsets behave predictably.
Ring Size Adjustment: Determine the desired ring size (inner diameter) from a conversion chart (e.g. US 6 → 16.5 mm
). Measure the mesh’s current inner diameter and compute a scale factor. Apply a uniform scale transform to the mesh to reach the target diameter.
Volume/Weight Calculation: Use a library like Trimesh to compute the model’s volume. For example, mesh.volume returns the current mesh volume
. Multiply by density to get weight.
Hollowing (Shell): If the weight exceeds the target, hollow out the model by offsetting inward. In code, use MeshLib’s mrmeshpy.offsetMesh(...) with a negative offset, or set params.type = Shell and a positive inner offset distance to create a hollow shell
. Pseudocode:
from meshlib import mrmeshpy
mesh = mrmeshpy.loadMesh("ring.stl") 
params = mrmeshpy.OffsetParameters()
params.type = mrmeshpy.OffsetParametersType.Shell
# e.g., hollow thickness 0.5mm
hollow_offset = 0.5  
hollowed = mrmeshpy.offsetMesh(mesh, -hollow_offset, params)
mrmeshpy.saveMesh(hollowed, "ring_hollow.stl")
This produces a ring with walls about 0.5 mm thick. Adjust hollow_offset until the computed weight is within range. (Alternatively, as noted, the pymadcad library’s thicken() function can do this in one call
.)
Iterate if Needed: Recompute volume of the hollowed mesh. If still too heavy, repeat with a larger offset; if too light, use a smaller offset or add internal infill structures. Each iteration involves re-running the offset operation and checking weight.
Finalize Model: Export the final mesh as glTF/GLB for viewing/printing. Ensure it is scaled to actual dimensions (units in millimeters).
This pipeline applies to any jewelry: rings, pendants, etc. For pendants or other shapes, the same offsetting will hollow out the interior. Additional parameters like overall thickness can also be exposed (by combining scaling and offset values) to give users fine-grained control.
Front-End UI & Visualization
On the front end (Next.js/React), use a 3D web viewer to display the model and allow interactive tweaks. A recommended approach is to use React Three Fiber (R3F) – a React renderer for Three.js. R3F lets us declaratively embed Three.js scenes in JSX, and it supports loading glTF models out of the box. In fact, as one tutorial notes, “everything you can make with raw Three.js will work in R3F,” and rendering performance is essentially the same as native Three.js
. In practice, you would write a React component that uses @react-three/fiber’s <Canvas> and the GLTFLoader. For example, using R3F’s hooks:
import { Canvas } from '@react-three/fiber';
import { useLoader } from '@react-three/fiber';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader';

function RingModel({ url, scale }) {
  const { scene } = useLoader(GLTFLoader, url);
  return <primitive object={scene} scale={[scale, scale, scale]} />;
}

function Scene({ modelUrl, modelScale }) {
  return (
    <Canvas>
      <ambientLight intensity={0.5} />
      <directionalLight position={[5,5,5]} />
      <RingModel url={modelUrl} scale={modelScale} />
    </Canvas>
  );
}
Here the modelUrl points to the generated .gltf file (which is an open 3D format)
. R3F’s <primitive> simply wraps the loaded Three.js scene object. For UI controls (sliders, input fields) we can use any React component library (e.g. Material-UI) or specialized 3D control libraries. One very convenient tool is Leva, which provides React-controlled GUI panels. Leva lets you quickly attach stateful sliders, checkboxes, color pickers, etc., directly to React state. For example, you can write const { ringSize, wallThickness } = useControls({ ringSize: 7, wallThickness: 0.5 }) and Leva will render UI controls that update those values in real time
. As the user drags these controls, you would trigger the backend to reprocess the model: sending the desired ringSize, wallThickness, and target weight, then fetching the updated mesh. In summary, the front-end workflow is:
Load the glTF model into the React scene with R3F.
Present form fields or sliders (using Leva or standard React inputs) for Ring Size, Target Weight, Wall Thickness, etc.
On change, call your Python backend API with the new parameters.
Receive the updated mesh (as glTF) and re-render it in the scene.
This creates an intuitive interface where a non-technical user simply enters the desired ring size and weight, then views the resulting 3D model in-browser. The heavy geometry work (scaling, offset) happens behind the scenes on the backend (using MeshLib, Trimesh, etc.), but from the user’s perspective it’s just clicking buttons or moving sliders. By combining a robust mesh-offset library (like MeshLib’s mrmeshpy) for the geometry and a React/Three.js frontend for interaction, we replicate the familiar “hollow/shell” feature of professional CAD tools (like Rhino) in a custom web app. All key sources confirm that voxel-based offset (via OpenVDB) and libraries like MeshLib are ideal for making hollow parts
, and that React Three Fiber is a modern, performant way to build 3D configurators on the web
. This approach satisfies the requirements: models of any ring size (via scaling) and any weight range (via controlled hollowing) can be generated with a minimal UI overhead for the user. Sources: MeshLib offsetting and shelling tools
; Trimesh volume property for weight
; standard ring size chart for diameters
; React Three Fiber usage and GLTF loading
; Leva GUI for controls
.
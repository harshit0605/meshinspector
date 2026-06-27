#include "MRViewer/MRStatePlugin.h"
#include "MRViewer/MRSurfaceManipulationWidget.h"
#include "MRViewer/MRRibbonRegisterItem.h"
#include "MRViewer/MRShowModal.h"
#include "MRViewer/MRUIStyle.h"
#include "MRMesh/MRObjectMesh.h"
#include "MRMesh/MRObjectsAccess.h"
#include "MRMesh/MRSceneRoot.h"

#include <imgui.h>
#include <string>
#include <utility>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

namespace MeshInspectorWorkbench
{

using namespace MR;

namespace
{

std::shared_ptr<ObjectMesh> getSelectedMesh_()
{
    auto meshes = getAllObjectsInTree<ObjectMesh>( &SceneRoot::get(), ObjectSelectivityType::Selected );
    if ( meshes.empty() )
        return {};
    return meshes.front();
}

class OfficialParityToolBase : public StatePlugin
{
public:
    OfficialParityToolBase(
        std::string name,
        StatePluginTabs tab,
        std::string tooltip,
        std::string status,
        bool enabled,
        std::string disabledReason = {} ) :
        StatePlugin( std::move( name ), tab ),
        tooltip_( std::move( tooltip ) ),
        status_( std::move( status ) ),
        enabled_( enabled ),
        disabledReason_( std::move( disabledReason ) )
    {
    }

    std::string getTooltip() const override
    {
        return tooltip_;
    }

protected:
    bool onEnable_() override
    {
        if ( !enabled_ )
        {
            showModal( disabledReason_.empty() ? "This official MeshInspector tool is disabled until its Rust backend operation is implemented." : disabledReason_, NotificationType::Warning );
            return false;
        }
        return StatePlugin::onEnable_();
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 360 * UI::scale() } ) )
            return;

        UI::transparentTextWrapped( "%s", tooltip_.c_str() );
        ImGui::Spacing();
        ImGui::BulletText( "Parity status: %s", status_.c_str() );
        ImGui::BulletText( "Geometry execution remains backend-owned through the hosted workbench bridge." );
        if ( !enabled_ )
            ImGui::BulletText( "Disabled reason: %s", disabledReason_.c_str() );
#ifdef __EMSCRIPTEN__
        const std::string dispatchId = dispatchCommandId_();
        if ( enabled_ && !dispatchId.empty() )
        {
            ImGui::Spacing();
            if ( ImGui::Button( "Run on backend" ) )
            {
                const std::string js =
                    "(function(){var cmd='" + dispatchId + "';var p=" + dispatchPayload_() + ";var w=window;"
                    "var b=w.MeshInspectorWorkbenchBridge;"
                    "if(b&&b.manifest){if(!b.manifest.command_capabilities)b.manifest.command_capabilities=[];"
                    "var caps=b.manifest.command_capabilities;var has=false;"
                    "for(var i=0;i<caps.length;i++){if(caps[i].command_id===cmd)has=true;}"
                    "if(!has){var o={};o.command_id=cmd;o.label=cmd;o.group='modify';o.rust_backed=true;caps.push(o);}}"
                    "if(typeof w.meshinspectorWorkbenchDispatchCommand==='function')"
                    "w.meshinspectorWorkbenchDispatchCommand(cmd,p);})();";
                emscripten_run_script( js.c_str() );
            }
        }
#endif
        ImGui::EndCustomStatePlugin();
    }

    virtual std::string dispatchCommandId_() const { return {}; }
    // JS object literal sent as the dispatch payload (workbench demo params); default empty.
    virtual std::string dispatchPayload_() const { return "{}"; }

private:
    std::string tooltip_;
    std::string status_;
    bool enabled_;
    std::string disabledReason_;
};

class DisabledOfficialParityToolBase : public OfficialParityToolBase
{
public:
    DisabledOfficialParityToolBase( std::string name, StatePluginTabs tab, std::string tooltip, std::string disabledReason ) :
        OfficialParityToolBase( std::move( name ), tab, std::move( tooltip ), "missing_backend_operation", false, std::move( disabledReason ) )
    {
    }
};

class FileSceneViewerTool final : public OfficialParityToolBase
{
public:
    FileSceneViewerTool() :
        OfficialParityToolBase(
            "File / Scene / Viewer",
            StatePluginTabs::Basic,
            "Official file, scene tree, viewer, history, and viewport controls mapped to the hosted workbench contract. PLY uploads route through Rust mesh_from_ply with MeshLib-style tri-corner polygon texcoord list packing, polygon face colors per source face row, and miniply-trimmed TextureFile comments.",
            "partial",
            true )
    {
    }
};

class MeshHealerTool final : public OfficialParityToolBase
{
public:
    MeshHealerTool() :
        OfficialParityToolBase(
            "Mesh Healer",
            StatePluginTabs::Mesh,
            "Mesh healing, hole filling, and production repair. Current backend support is partial and Rust-backed for repair diagnostics, MultipleEdgesResolveMode None/Simple/Strong dispatch, Simple-mode duplicate-edge avoidance, Strong-mode reused generated chord repair, outNewFaces new-face index reporting, maxPolygonSubdivisions split sampling, makeDegenerateBand duplicate-boundary band creation, stopBeforeBadTriangulation bad-patch guarding, smoothBd boundary-edge metric control, getMinAreaMetric double-area triangulation, getEdgeLengthFillMetric edge-length triangulation, getUniversalMetric universal smooth triangulation, getMaxDihedralAngleMetric max-dihedral-angle triangulation, getParallelPlaneFillMetric parallel-plane projection triangulation, getComplexFillMetric aspect-area edge-penalty triangulation, getMinTriAngleMetric minimum-angle triangulation, getPlaneFillMetric plane-normal triangulation, getPlaneNormalizedFillMetric plane-normalized aspect triangulation, getComplexStitchMetric aspect-ratio/dihedral stitch triangulation, getEdgeLengthStitchMetric edge-length stitch triangulation, getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation, and getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation during service hole filling, crease detection, crease repair planning and execution, boundary-only close-vertex uniting, crease component/branch filtering, not-smooth face detection, MeshBuilder-style non-manifold edge face-pruning repair, MeshBuilder-style duplicateNonManifoldVertices disconnected, repeated-neighbor, face-region scoped, partial-triangulation lastValidVert duplicate-id allocation, and single-pass path-orientation behavior, SelfIntersections::getFaces strict non-touching face detection, SelfIntersections::fix Relax topology-preserving repair with subdivision disabled, Rust topological tunnel diagnostics including MeshLib-oracle 24x8/24x10/24x12 torus detectTunnelFaces bands, SDF rebuild self-intersection repair, and basic repair.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "repair"; }
};

class MeshEditSimplifyTool final : public OfficialParityToolBase
{
public:
    MeshEditSimplifyTool() :
        OfficialParityToolBase(
            "Mesh Edit / Simplify",
            StatePluginTabs::Mesh,
            "Official mesh edit, smoothing, simplification, subdivision, and deformation surface. Resize, scoop, smooth, Rust-backed shortest-edge decimation, and subdivision slices are currently wired.",
            "partial",
            true )
    {
    }
};

class DecimateMeshTool final : public OfficialParityToolBase
{
public:
    DecimateMeshTool() :
        OfficialParityToolBase(
            "Decimate Mesh",
            StatePluginTabs::Mesh,
            "Rust-backed MeshLib MR::decimateMesh DecimateStrategy::MinimizeError QEM with target triangle count/percentage stop controls through maxDeletedFaces, stabilizer and angleWeightedDistToPlane face-plane weighting and ShortestEdgeFirst subset for maxError stop behavior, FaceBitSet region masks, deletion limits including MeshLib's unbounded-default half-face guard, maxEdgeLen, maxBdShift boundary-shift guards, maxTriangleAspectRatio guards, criticalTriAspectRatio aspect-relaxation guard, tinyEdgeLength endpoint aspect-bypass guard, maxAngleChange local Delone flip guard, touchNearBdEdges boundary filtering, touchBdVerts boundary-vertex preservation, notFlippable adjacent-collapse guards with crease-form QEM weighting, optimized collapse positions, notFlippable dynamic remapping with remapped_not_flippable_edges metadata, edgesToCollapse collapse subset and remapping metadata, twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, and collapse/flip/pack remapping metadata, MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation, packMesh output, subdivideParts part partitioning, and decimateBetweenParts final pass. arbitrary preCollapse callbacks and true threaded execution remain parity work.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "decimate-mesh"; }
    std::string dispatchPayload_() const override { return "{max_error:1000}"; }
};

class SubdivideMeshTool final : public OfficialParityToolBase
{
public:
    SubdivideMeshTool() :
        OfficialParityToolBase(
            "Subdivide Mesh",
            StatePluginTabs::Mesh,
            "Rust-backed MeshLib-style subdivision for maxEdgeLen, FaceBitSet region masks, notFlippable protected Delone-ring edge guards with split-edge remapping, maxDeviationAfterFlip, maxAngleChangeAfterFlip, criticalAspectRatioFlip, curvaturePriority, maxEdgeSplits, maxTriAspectRatio, maxSplittableTriAspectRatio, projectOnOriginalMesh, smoothMode cotan positioning, and chained local Delone topology. Broader smoothMode crease-topology oracles remain parity work.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "subdivide-mesh"; }
};

class MakeDeloneTool final : public OfficialParityToolBase
{
public:
    MakeDeloneTool() :
        OfficialParityToolBase(
            "Make Delone",
            StatePluginTabs::Mesh,
            "Rust-backed MeshLib MR::makeDeloneEdgeFlips local Delone edge-flip pass with region face masks, iteration control, maxDeviationAfterFlip diagonal-deviation guard, maxAngleChange dihedral-delta guard, criticalTriAspectRatio angle-guard override, notFlippable edge constraints, and vertRegion vertex constraints.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "make-delone"; }
};

class BooleanCollisionTool final : public DisabledOfficialParityToolBase
{
public:
    BooleanCollisionTool() :
        DisabledOfficialParityToolBase(
            "Boolean / Collision",
            StatePluginTabs::Mesh,
            "Exact and voxel boolean operations plus collision detection. Exact boolean, voxel boolean, and collision detection are exposed separately as Rust-backed commands.",
            "The aggregate boolean tool is disabled until combined official boolean workflow parity gates pass." )
    {
    }
};

class ExactBooleanTool final : public OfficialParityToolBase
{
public:
    ExactBooleanTool() :
        OfficialParityToolBase(
            "Exact Boolean",
            StatePluginTabs::Mesh,
            "Run MeshLib MR::boolean-style exact union, intersection, difference, inside, and outside operations against another ready version through the Rust-backed SDK.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "exact-boolean"; }
};

class VoxelBooleanTool final : public OfficialParityToolBase
{
public:
    VoxelBooleanTool() :
        OfficialParityToolBase(
            "Voxel Boolean",
            StatePluginTabs::Mesh,
            "Run MeshLib MRVoxels-style voxel union, intersection, or difference against another ready version through the Rust-backed SDK.",
            "partial",
            true )
    {
    }
};

class CollisionDetectionTool final : public OfficialParityToolBase
{
public:
    CollisionDetectionTool() :
        OfficialParityToolBase(
            "Collision Detection",
            StatePluginTabs::Mesh,
            "Find MeshLib findCollidingTriangles-style exact face pairs between the active mesh and another ready version through the Rust-backed SDK.",
            "partial",
            true )
    {
    }
};

class OffsetShellTool final : public OfficialParityToolBase
{
public:
    OffsetShellTool() :
        OfficialParityToolBase(
            "Offset / Shell",
            StatePluginTabs::Mesh,
            "Offset, shell, thickening, weighted shell, partial offset, offset verts, expand/shrink, shrink/expand, and the separate Offset Contours command are exposed through Rust kernels; broader contour-index hardening remains tracked.",
            "partial",
            true )
    {
    }
};

class OffsetMeshTool final : public OfficialParityToolBase
{
public:
    OffsetMeshTool() :
        OfficialParityToolBase(
            "Offset Mesh",
            StatePluginTabs::Mesh,
            "Run the official Offset tool's entire-model offset mode through the Rust voxel offset kernel.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "offset-mesh"; }
    std::string dispatchPayload_() const override { return "{offset_mm:0.1}"; }
};

class ShellMeshTool final : public OfficialParityToolBase
{
public:
    ShellMeshTool() :
        OfficialParityToolBase(
            "Shell Mesh",
            StatePluginTabs::Mesh,
            "Run the official Offset tool's shell mode through the Rust voxel shell kernel.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "shell-mesh"; }
    std::string dispatchPayload_() const override { return "{offset_mm:0.1}"; }
};

class ThickeningTool final : public OfficialParityToolBase
{
public:
    ThickeningTool() :
        OfficialParityToolBase(
            "Thickening",
            StatePluginTabs::Mesh,
            "Run MeshLib thickenMesh signed-thickness mode through the Rust voxel thickening kernel.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "thicken-mesh"; }
    std::string dispatchPayload_() const override { return "{thickness_mm:0.4}"; }
};

class WeightedShellTool final : public OfficialParityToolBase
{
public:
    WeightedShellTool() :
        OfficialParityToolBase(
            "Weighted Shell",
            StatePluginTabs::Mesh,
            "Run MeshLib WeightedShell::meshShell with selected-region additive offsets through the Rust voxel shell kernel.",
            "partial",
            true )
    {
    }
};

class PartialOffsetTool final : public OfficialParityToolBase
{
public:
    PartialOffsetTool() :
        OfficialParityToolBase(
            "Partial Offset",
            StatePluginTabs::Mesh,
            "Run MeshLib partialOffsetMesh with selected-region unsigned offset and union semantics through the Rust voxel partial-offset kernel.",
            "partial",
            true )
    {
    }
};

class OffsetVertsTool final : public OfficialParityToolBase
{
public:
    OffsetVertsTool() :
        OfficialParityToolBase(
            "Offset Verts",
            StatePluginTabs::Mesh,
            "Run MeshLib MR::offsetVerts pseudonormal vertex shifting through the Rust mesh-edit kernel.",
            "partial",
            true )
    {
    }
};

class ExpandShrinkTool final : public OfficialParityToolBase
{
public:
    ExpandShrinkTool() :
        OfficialParityToolBase(
            "Expand/Shrink",
            StatePluginTabs::Mesh,
            "Run the official Offset tool's concave-feature smoothing mode through two signed Rust voxel offset passes.",
            "partial",
            true )
    {
    }
};

class ShrinkExpandTool final : public OfficialParityToolBase
{
public:
    ShrinkExpandTool() :
        OfficialParityToolBase(
            "Shrink/Expand",
            StatePluginTabs::Mesh,
            "Run the official Offset tool's convex-feature smoothing mode through two signed Rust voxel offset passes.",
            "partial",
            true )
    {
    }
};

class CompareReportTool final : public OfficialParityToolBase
{
public:
    CompareReportTool() :
        OfficialParityToolBase(
            "Compare / Report",
            StatePluginTabs::Analysis,
            "Deviation comparison, signed distance fields, and QA report workflow.",
            "partial",
            true )
    {
    }

    std::string dispatchCommandId_() const override { return "compare-versions"; }
};

class PointCloudIcpTool final : public OfficialParityToolBase
{
public:
    PointCloudIcpTool() :
        OfficialParityToolBase(
            "Point Cloud / ICP",
            StatePluginTabs::PointCloud,
            "Point clouds, scan alignment, triangulation, and ICP. Rust-backed point-cloud projections, single-mesh projection payloads with rigid object/reference transforms, face-region masks, and face/edge/vertex pseudonormal normals, neighbor queries, local fan boundary detection/optimization, repeated-triangle mesh assembly, MeshBuilder-style half-edge origin-ring insertion guards, small-hole fill thresholds with MultipleEdgesResolveMode None/Simple/Strong dispatch, Simple-mode duplicate-edge avoidance, Strong-mode reused generated chord repair, outNewFaces new-face index reporting, maxPolygonSubdivisions split sampling, makeDegenerateBand duplicate-boundary band creation, stopBeforeBadTriangulation bad-patch guarding, smoothBd boundary-edge metric control, getMinAreaMetric double-area triangulation, getEdgeLengthFillMetric edge-length triangulation, getUniversalMetric universal smooth triangulation, getMaxDihedralAngleMetric max-dihedral-angle triangulation, getParallelPlaneFillMetric parallel-plane projection triangulation, getComplexFillMetric aspect-area edge-penalty triangulation, getMinTriAngleMetric minimum-angle triangulation, getPlaneFillMetric plane-normal triangulation, getPlaneNormalizedFillMetric plane-normalized aspect triangulation, getComplexStitchMetric aspect-ratio/dihedral stitch triangulation, getEdgeLengthStitchMetric edge-length stitch triangulation, getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation, and getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation, uniform/grid sampling, pairwise point-to-point/point-to-plane ICP, MeshLib maxGroupSize=1-style independent multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize=0-style all-object multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize>1 sequential cascade multiway point-to-point/point-to-plane/combined ICP, and MeshLib AABBTreeBased cascade multiway point-to-point/point-to-plane/combined ICP are available; full MeshLib mesh-topology materialization, arbitrary callback FillHoleMetric parameterization, and non-rigid tree-accelerated/multi-object mesh projection workflows remain open.",
            "partial",
            true )
    {
    }
};

class VoxelsCtSdfTool final : public DisabledOfficialParityToolBase
{
public:
    VoxelsCtSdfTool() :
        DisabledOfficialParityToolBase(
            "Voxels / CT / SDF",
            StatePluginTabs::Voxels,
            "Voxel volumes, CT reconstruction, SDF, and marching extraction. Rust kernels exist but official product UI is not exposed.",
            "SDF and voxel kernels are backend-only today; voxel-object and CT workflows are not product commands yet." )
    {
    }
};

class MeshToVoxelsSdfTool final : public OfficialParityToolBase
{
public:
    MeshToVoxelsSdfTool() :
        OfficialParityToolBase(
            "Mesh to Voxels / SDF",
            StatePluginTabs::Voxels,
            "Convert the selected mesh to a MeshLib meshToLevelSet or meshToDistanceField-style voxel distance field through the Rust-backed SDK.",
            "partial",
            true )
    {
    }
};

class OpenRawVoxelsTool final : public OfficialParityToolBase
{
public:
    OpenRawVoxelsTool() :
        OfficialParityToolBase(
            "Open RAW Voxels",
            StatePluginTabs::Voxels,
            "Load MeshLib VoxelsLoad::fromRaw explicit or filename-auto RAW voxel payloads through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class OpenVoxelsFromTiffTool final : public OfficialParityToolBase
{
public:
    OpenVoxelsFromTiffTool() :
        OfficialParityToolBase(
            "Open Voxels From TIFF",
            StatePluginTabs::Voxels,
            "Load MeshLib VoxelsLoad::loadTiffDir TIFF slice stacks through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class VoxelsSliceTool final : public OfficialParityToolBase
{
public:
    VoxelsSliceTool() :
        OfficialParityToolBase(
            "Voxels Slice",
            StatePluginTabs::Voxels,
            "Voxel slice extraction is Rust SDK-backed for MeshLib marked-slice image semantics and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsLineGraphTool final : public OfficialParityToolBase
{
public:
    VoxelsLineGraphTool() :
        OfficialParityToolBase(
            "Voxels Line Graph",
            StatePluginTabs::Voxels,
            "Voxel line-graph sampling is Rust SDK-backed for CT axis probes and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class SetActiveVoxelBoxTool final : public OfficialParityToolBase
{
public:
    SetActiveVoxelBoxTool() :
        OfficialParityToolBase(
            "Set Active Voxel Box",
            StatePluginTabs::Voxels,
            "Active voxel-box cropping is Rust SDK-backed with MeshLib max-excluded bounds semantics and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsVolumeRenderingDataTool final : public OfficialParityToolBase
{
public:
    VoxelsVolumeRenderingDataTool() :
        OfficialParityToolBase(
            "Voxels Volume Rendering Data",
            StatePluginTabs::Voxels,
            "ObjectVoxels volume-rendering data preparation is Rust SDK-backed and executable through the hosted workbench bridge. Full GL viewport controls remain parity work.",
            "partial",
            true )
    {
    }
};

class VoxelsVolumeRenderingLutTool final : public OfficialParityToolBase
{
public:
    VoxelsVolumeRenderingLutTool() :
        OfficialParityToolBase(
            "Voxels Volume Rendering LUT",
            StatePluginTabs::Voxels,
            "RenderVolumeObject denseMap transfer-function generation is Rust SDK-backed and executable through the hosted workbench bridge. Full GL viewport controls remain parity work.",
            "partial",
            true )
    {
    }
};

class VoxelsVolumeRenderingRayTool final : public OfficialParityToolBase
{
public:
    VoxelsVolumeRenderingRayTool() :
        OfficialParityToolBase(
            "Voxels Volume Rendering Ray",
            StatePluginTabs::Voxels,
            "MRVolumeShader ray compositing is Rust SDK-backed and executable through the hosted workbench bridge. Full GL viewport controls remain parity work.",
            "partial",
            true )
    {
    }
};

class VoxelsSegmentationTool final : public OfficialParityToolBase
{
public:
    VoxelsSegmentationTool() :
        OfficialParityToolBase(
            "Voxels Segmentation",
            StatePluginTabs::Voxels,
            "Voxel graph-cut segmentation is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsMaskToMeshTool final : public OfficialParityToolBase
{
public:
    VoxelsMaskToMeshTool() :
        OfficialParityToolBase(
            "Voxels Mask to Mesh",
            StatePluginTabs::Voxels,
            "Voxel mask-to-mesh conversion is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsToMeshSimpleTool final : public OfficialParityToolBase
{
public:
    VoxelsToMeshSimpleTool() :
        OfficialParityToolBase(
            "Voxels to Mesh Simple",
            StatePluginTabs::Voxels,
            "Simple voxel-to-mesh conversion is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsToMeshDualTool final : public OfficialParityToolBase
{
public:
    VoxelsToMeshDualTool() :
        OfficialParityToolBase(
            "Voxels to Mesh Dual",
            StatePluginTabs::Voxels,
            "Dual voxel-to-mesh conversion is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsToMeshSmartTool final : public OfficialParityToolBase
{
public:
    VoxelsToMeshSmartTool() :
        OfficialParityToolBase(
            "Voxels to Mesh Smart",
            StatePluginTabs::Voxels,
            "Smart voxel-to-mesh refinement is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsPathTool final : public OfficialParityToolBase
{
public:
    VoxelsPathTool() :
        OfficialParityToolBase(
            "Voxels Path",
            StatePluginTabs::Voxels,
            "Voxel path construction is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class VoxelsPathBuildFourTool final : public OfficialParityToolBase
{
public:
    VoxelsPathBuildFourTool() :
        OfficialParityToolBase(
            "Voxels Path Build Four",
            StatePluginTabs::Voxels,
            "Voxel path Build Four mode is Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class BinaryOperationsTool final : public OfficialParityToolBase
{
public:
    BinaryOperationsTool() :
        OfficialParityToolBase(
            "Binary Operations",
            StatePluginTabs::Voxels,
            "Voxel binary operations are Rust SDK-backed and executable through the hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class DistanceMapsLinesGcodeTool final : public DisabledOfficialParityToolBase
{
public:
    DistanceMapsLinesGcodeTool() :
        DisabledOfficialParityToolBase(
            "Distance Maps / Lines / G-code",
            StatePluginTabs::DistanceMap,
            "Rust-backed mesh and contour distance maps, closed-contour offset slice, ObjectLines, TIFF import/export, iso-lines, merge, contour boolean composition, and G-code strtof-narrowed, CRLF-preserving, no-motion-aware path/source workflows are available.",
            "Distance-map kernels and ObjectLines file import/export kernels are Rust SDK-backed but do not yet have full product input/output workflows. Use the enabled Mesh Distance Map, Contour Distance Map, Distance Map Iso-Lines, Distance Map Merge, Contour Boolean, Offset Contours, ObjectLines contour conversion, and G-code tools for current product paths." )
    {
    }
};

class DistanceMapFromMeshTool final : public OfficialParityToolBase
{
public:
    DistanceMapFromMeshTool() :
        OfficialParityToolBase(
            "Mesh Distance Map",
            StatePluginTabs::DistanceMap,
            "Compute MeshLib computeDistanceMap-style samples from the active mesh through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapContoursTool final : public OfficialParityToolBase
{
public:
    DistanceMapContoursTool() :
        OfficialParityToolBase(
            "Contour Distance Map",
            StatePluginTabs::DistanceMap,
            "Compute MeshLib DistanceMap samples from contour polylines through the Rust distance_map_from_contours SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapIsoLinesTool final : public OfficialParityToolBase
{
public:
    DistanceMapIsoLinesTool() :
        OfficialParityToolBase(
            "Distance Map Iso-Lines",
            StatePluginTabs::DistanceMap,
            "Extract MeshLib distanceMapTo2DIsoPolyline-style iso-line segments from DistanceMap samples through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapMergeTool final : public OfficialParityToolBase
{
public:
    DistanceMapMergeTool() :
        OfficialParityToolBase(
            "Distance Map Merge",
            StatePluginTabs::DistanceMap,
            "Merge MeshLib DistanceMap samples with max, min, and subtraction invalid-cell semantics through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapContourBooleanTool final : public OfficialParityToolBase
{
public:
    DistanceMapContourBooleanTool() :
        OfficialParityToolBase(
            "Contour Boolean From Distance Maps",
            StatePluginTabs::DistanceMap,
            "Compose closed contours with MeshLib contourUnion, contourIntersection, and contourSubtract signed-distance semantics through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapFromTiffTool final : public OfficialParityToolBase
{
public:
    DistanceMapFromTiffTool() :
        OfficialParityToolBase(
            "TIFF Distance Map Import",
            StatePluginTabs::DistanceMap,
            "Load MeshLib DistanceMapLoad::fromTiff-style GeoTIFF distance maps through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class DistanceMapToTiffTool final : public OfficialParityToolBase
{
public:
    DistanceMapToTiffTool() :
        OfficialParityToolBase(
            "TIFF Distance Map Export",
            StatePluginTabs::DistanceMap,
            "Write MeshLib DistanceMapSave::toTiff-style GeoTIFF distance maps through the Rust SDK endpoint.",
            "partial",
            true )
    {
    }
};

class OffsetContoursTool final : public OfficialParityToolBase
{
public:
    OffsetContoursTool() :
        OfficialParityToolBase(
            "Offset Contours",
            StatePluginTabs::DistanceMap,
            "Run MeshLib MROffsetContours closed signed Type::Offset, sharp max-angle 3D Z restore with explicit relaxIterations, constant zCallback-equivalent restore, fixed and variable positive/inward/shell origin maps, default 3D signed/shell Z restore/relaxation, signed variable round/sharp Type::Offset, signed variable round/sharp shell, open end, variable open-end behavior, direction-reversed horizontal collinear-overlap with first-source and both-reversed ordering, vertical direction variants, diagonal direction variants, and three-segment horizontal/vertical/diagonal collinear-overlap chains including diagonal chain direction variants global-outline indicesMap/origin output through the Rust line kernel.",
            "partial",
            true )
    {
    }
};

class ObjectLinesFromContoursTool final : public OfficialParityToolBase
{
public:
    ObjectLinesFromContoursTool() :
        OfficialParityToolBase(
            "ObjectLines From Contours",
            StatePluginTabs::DistanceMap,
            "Build MeshLib ObjectLines/ObjectLinesHolder scene JSON from contour polylines through the Rust PolylineTopology-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesLoadMrLinesTool final : public OfficialParityToolBase
{
public:
    ObjectLinesLoadMrLinesTool() :
        OfficialParityToolBase(
            "ObjectLines Load MrLines",
            StatePluginTabs::DistanceMap,
            "Load MeshLib LinesLoad::fromMrLines binary PolylineTopology and Vector3f point payloads through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesSaveMrLinesTool final : public OfficialParityToolBase
{
public:
    ObjectLinesSaveMrLinesTool() :
        OfficialParityToolBase(
            "ObjectLines Save MrLines",
            StatePluginTabs::DistanceMap,
            "Export ObjectLines to MeshLib LinesSave::toMrLines binary PolylineTopology and Vector3f point payloads through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesLoadPlyTool final : public OfficialParityToolBase
{
public:
    ObjectLinesLoadPlyTool() :
        OfficialParityToolBase(
            "ObjectLines Load PLY",
            StatePluginTabs::DistanceMap,
            "Load MeshLib LinesLoad::fromPly vertex and edge payloads, including edge elements without vertex1/vertex2 skipping, through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesSavePlyTool final : public OfficialParityToolBase
{
public:
    ObjectLinesSavePlyTool() :
        OfficialParityToolBase(
            "ObjectLines Save PLY",
            StatePluginTabs::DistanceMap,
            "Export ObjectLines to MeshLib LinesSave::toPly binary little-endian vertex, optional color, and edge payloads through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesLoadPtsTool final : public OfficialParityToolBase
{
public:
    ObjectLinesLoadPtsTool() :
        OfficialParityToolBase(
            "ObjectLines Load PTS",
            StatePluginTabs::DistanceMap,
            "Load MeshLib LinesLoad::fromPts BEGIN_Polyline/END_Polyline text with Vector3f coordinate narrowing, trailing point-field tolerance, and last-coordinate numeric-prefix suffix tolerance into ObjectLines through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesSavePtsTool final : public OfficialParityToolBase
{
public:
    ObjectLinesSavePtsTool() :
        OfficialParityToolBase(
            "ObjectLines Save PTS",
            StatePluginTabs::DistanceMap,
            "Export ObjectLines to MeshLib LinesSave::toPts BEGIN_Polyline/END_Polyline text through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesLoadSvgTool final : public OfficialParityToolBase
{
public:
    ObjectLinesLoadSvgTool() :
        OfficialParityToolBase(
            "ObjectLines Load SVG",
            StatePluginTabs::DistanceMap,
            "Load MeshLib MRIOExtras LinesLoad::fromSvg line, compact signed polyline/polygon points, shape, path, and transform geometry into ObjectLines through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesSaveDxfTool final : public OfficialParityToolBase
{
public:
    ObjectLinesSaveDxfTool() :
        OfficialParityToolBase(
            "ObjectLines Save DXF",
            StatePluginTabs::DistanceMap,
            "Export ObjectLines to MeshLib LinesSave::toDxf POLYLINE/VERTEX/SEQEND text through the Rust-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class ObjectLinesToContoursTool final : public OfficialParityToolBase
{
public:
    ObjectLinesToContoursTool() :
        OfficialParityToolBase(
            "ObjectLines To Contours",
            StatePluginTabs::DistanceMap,
            "Restore contour polylines from MeshLib ObjectLines/ObjectLinesHolder scene JSON through the Rust PolylineTopology-backed SDK endpoint.",
            "partial",
            true )
    {
    }
};

class GcodePathParserTool final : public OfficialParityToolBase
{
public:
    GcodePathParserTool() :
        OfficialParityToolBase(
            "G-code Path Parser",
            StatePluginTabs::DistanceMap,
            "Parse G-code source frames into MeshLib GcodeProcessor-style toolpath segments with strtof command-value narrowing including leading command-value whitespace, special, and hexadecimal float tokens, no-motion feedrateMax updates, zero-idle feedrate post-pass rewriting, radius-only G2/G3 no-op handling, G28 home zero-length idle actions, and MeshLib-style arc radius-mismatch warnings through the Rust-backed SDK.",
            "partial",
            true )
    {
    }
};

class GcodeLoadSourceTool final : public OfficialParityToolBase
{
public:
    GcodeLoadSourceTool() :
        OfficialParityToolBase(
            "Load G-code Source",
            StatePluginTabs::DistanceMap,
            "Load MeshLib-supported G-code source frames with CRLF carriage-return preservation through the Rust-backed SDK and hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class GcodeWriteSourceTool final : public OfficialParityToolBase
{
public:
    GcodeWriteSourceTool() :
        OfficialParityToolBase(
            "Write G-code Source",
            StatePluginTabs::DistanceMap,
            "Write MeshLib ObjectGcode-style source frames through the Rust-backed SDK and hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class GcodeParseFilePathsTool final : public OfficialParityToolBase
{
public:
    GcodeParseFilePathsTool() :
        OfficialParityToolBase(
            "Parse G-code File Paths",
            StatePluginTabs::DistanceMap,
            "Parse MeshLib-supported G-code source files with CRLF carriage-return frame preservation into GcodeProcessor-style toolpaths through the Rust-backed SDK and hosted workbench bridge.",
            "partial",
            true )
    {
    }
};

class AutomationPluginApiTool final : public OfficialParityToolBase
{
public:
    AutomationPluginApiTool() :
        OfficialParityToolBase(
            "Automation / Plugin API",
            StatePluginTabs::Other,
            "Hosted workbench manifest, backend command bridge, and plugin automation integration.",
            "partial",
            true )
    {
    }
};

class SurfaceBrushToolBase : public StatePlugin
{
public:
    SurfaceBrushToolBase( std::string name, SurfaceManipulationWidget::WorkMode mode, std::string tooltip ) :
        StatePlugin( std::move( name ), StatePluginTabs::Mesh ),
        mode_( mode ),
        tooltip_( std::move( tooltip ) )
    {
        settings_.workMode = mode_;
        settings_.radius = 1.0f;
        settings_.editForce = 0.3f;
        settings_.relaxForce = 0.2f;
        settings_.sharpness = 55.0f;
        settings_.relaxForceAfterEdit = 0.1f;
    }

    std::string getTooltip() const override
    {
        return tooltip_;
    }

protected:
    bool onEnable_() override
    {
        if ( !StatePlugin::onEnable_() )
            return false;
        target_ = getSelectedMesh_();
        if ( !target_ )
        {
            showModal( "Select a mesh object before activating an interactive brush.", NotificationType::Warning );
            return false;
        }

        widget_.reset();
        widget_.init( target_ );
        widget_.setSettings( settings_ );
        widget_.enableDeviationVisualization( true );
        return true;
    }

    bool onDisable_() override
    {
        widget_.reset();
        target_.reset();
        return StatePlugin::onDisable_();
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 320 * UI::scale() } ) )
            return;

        UI::transparentTextWrapped(
            "This tool uses MeshLib's SurfaceManipulationWidget for direct mouse-driven editing in the viewport."
        );
        ImGui::Spacing();
        ImGui::SliderFloat( "Brush radius (mm)", &settings_.radius, 0.2f, 6.0f, "%.2f" );
        ImGui::SliderFloat( "Edit force", &settings_.editForce, 0.05f, 2.0f, "%.2f" );
        ImGui::SliderFloat( "Sharpness", &settings_.sharpness, 0.0f, 100.0f, "%.1f" );
        ImGui::SliderFloat( "Relax after edit", &settings_.relaxForceAfterEdit, 0.0f, 0.5f, "%.2f" );
        bool ignoreOcclusion = widget_.ignoreOcclusion();
        if ( ImGui::Checkbox( "Ignore occlusion", &ignoreOcclusion ) )
            widget_.setIgnoreOcclusion( ignoreOcclusion );

        bool codirectedOnly = widget_.isEditOnlyCodirectedSurface();
        if ( ImGui::Checkbox( "Only edit co-directed surface", &codirectedOnly ) )
            widget_.setEditOnlyCodirectedSurface( codirectedOnly );

        if ( mode_ == SurfaceManipulationWidget::WorkMode::Relax )
            ImGui::SliderFloat( "Relax force", &settings_.relaxForce, 0.01f, 0.5f, "%.2f" );

        widget_.setSettings( settings_ );

        ImGui::Spacing();
        UI::separator();
        UI::transparentTextWrapped(
            "Mouse workflow: activate tool, paint directly on the mesh, then commit the edited mesh through the web host bridge."
        );
        ImGui::EndCustomStatePlugin();
    }

private:
    SurfaceManipulationWidget widget_;
    SurfaceManipulationWidget::Settings settings_;
    std::shared_ptr<ObjectMesh> target_;
    SurfaceManipulationWidget::WorkMode mode_;
    std::string tooltip_;
};

class ThickenBrushTool final : public SurfaceBrushToolBase
{
public:
    ThickenBrushTool() :
        SurfaceBrushToolBase(
            "Thicken Brush",
            SurfaceManipulationWidget::WorkMode::Add,
            "Pushes the surface outward under the cursor using MeshLib surface manipulation."
        )
    {
    }
};

class ScoopBrushTool final : public SurfaceBrushToolBase
{
public:
    ScoopBrushTool() :
        SurfaceBrushToolBase(
            "Scoop Brush",
            SurfaceManipulationWidget::WorkMode::Remove,
            "Pushes the surface inward under the cursor for local recess and cavity work."
        )
    {
    }
};

class SmoothBrushTool final : public SurfaceBrushToolBase
{
public:
    SmoothBrushTool() :
        SurfaceBrushToolBase(
            "Smooth Brush",
            SurfaceManipulationWidget::WorkMode::Relax,
            "Relaxes rough AI-generated chatter under the brush while preserving the surrounding surface."
        )
    {
    }
};

class RegionMarkTool final : public StatePlugin
{
public:
    RegionMarkTool() : StatePlugin( "Select / Mark Region", StatePluginTabs::Selection ) {}

    std::string getTooltip() const override
    {
        return "Selection bridge for painted, lassoed, rectangular, semantic, and MeshLib selector edit masks.";
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 320 * UI::scale() } ) )
            return;
        UI::transparentTextWrapped(
            "This tool is the placeholder for surface, face, brush, rectangle, and selector mask capture. The web host serializes these masks into MeshInspector's interactive selection payload."
        );
        ImGui::BulletText( "Brush-paint mask capture" );
        ImGui::BulletText( "Lasso / rectangle capture" );
        ImGui::BulletText( "Semantic region fallback" );
        ImGui::EndCustomStatePlugin();
    }
};

class MeasureInspectTool final : public StatePlugin
{
public:
    MeasureInspectTool() : StatePlugin( "Measure / Inspect", StatePluginTabs::Analysis ) {}

    std::string getTooltip() const override
    {
        return "Cursor-space inspection hook for hover coordinates, point-to-point measures, and local thickness probes.";
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 320 * UI::scale() } ) )
            return;
        UI::transparentTextWrapped(
            "Use MeshLib pickers to expose cursor point info, local normals, thickness probes, and point-to-point measurement in the hosted workbench."
        );
        ImGui::EndCustomStatePlugin();
    }
};

class MeshCutMeasurePathTool final : public StatePlugin
{
public:
    MeshCutMeasurePathTool() : StatePlugin( "Mesh Cut & Measure Path", StatePluginTabs::Analysis ) {}

    std::string getTooltip() const override
    {
        return "Builds and measures a MeshLib geodesic cut path and exports it as ObjectLines and ObjectPoints path objects.";
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 320 * UI::scale() } ) )
            return;
        UI::transparentTextWrapped(
            "Use MeshLib shortest-surface path controls to create a cut contour, measure its length, and export the path as Polyline ObjectLines or PointCloud ObjectPoints scene objects."
        );
        ImGui::EndCustomStatePlugin();
    }
};

}

MR_REGISTER_RIBBON_ITEM( FileSceneViewerTool )
MR_REGISTER_RIBBON_ITEM( RegionMarkTool )
MR_REGISTER_RIBBON_ITEM( MeshHealerTool )
MR_REGISTER_RIBBON_ITEM( MeshEditSimplifyTool )
MR_REGISTER_RIBBON_ITEM( DecimateMeshTool )
MR_REGISTER_RIBBON_ITEM( SubdivideMeshTool )
MR_REGISTER_RIBBON_ITEM( MakeDeloneTool )
MR_REGISTER_RIBBON_ITEM( ThickenBrushTool )
MR_REGISTER_RIBBON_ITEM( ScoopBrushTool )
MR_REGISTER_RIBBON_ITEM( SmoothBrushTool )
MR_REGISTER_RIBBON_ITEM( BooleanCollisionTool )
MR_REGISTER_RIBBON_ITEM( ExactBooleanTool )
MR_REGISTER_RIBBON_ITEM( VoxelBooleanTool )
MR_REGISTER_RIBBON_ITEM( CollisionDetectionTool )
MR_REGISTER_RIBBON_ITEM( OffsetShellTool )
MR_REGISTER_RIBBON_ITEM( OffsetMeshTool )
MR_REGISTER_RIBBON_ITEM( ShellMeshTool )
MR_REGISTER_RIBBON_ITEM( ThickeningTool )
MR_REGISTER_RIBBON_ITEM( WeightedShellTool )
MR_REGISTER_RIBBON_ITEM( PartialOffsetTool )
MR_REGISTER_RIBBON_ITEM( OffsetVertsTool )
MR_REGISTER_RIBBON_ITEM( ExpandShrinkTool )
MR_REGISTER_RIBBON_ITEM( ShrinkExpandTool )
MR_REGISTER_RIBBON_ITEM( MeasureInspectTool )
MR_REGISTER_RIBBON_ITEM( MeshCutMeasurePathTool )
MR_REGISTER_RIBBON_ITEM( CompareReportTool )
MR_REGISTER_RIBBON_ITEM( PointCloudIcpTool )
MR_REGISTER_RIBBON_ITEM( VoxelsCtSdfTool )
MR_REGISTER_RIBBON_ITEM( MeshToVoxelsSdfTool )
MR_REGISTER_RIBBON_ITEM( OpenRawVoxelsTool )
MR_REGISTER_RIBBON_ITEM( OpenVoxelsFromTiffTool )
MR_REGISTER_RIBBON_ITEM( VoxelsSliceTool )
MR_REGISTER_RIBBON_ITEM( VoxelsLineGraphTool )
MR_REGISTER_RIBBON_ITEM( SetActiveVoxelBoxTool )
MR_REGISTER_RIBBON_ITEM( VoxelsVolumeRenderingDataTool )
MR_REGISTER_RIBBON_ITEM( VoxelsVolumeRenderingLutTool )
MR_REGISTER_RIBBON_ITEM( VoxelsVolumeRenderingRayTool )
MR_REGISTER_RIBBON_ITEM( VoxelsSegmentationTool )
MR_REGISTER_RIBBON_ITEM( VoxelsMaskToMeshTool )
MR_REGISTER_RIBBON_ITEM( VoxelsToMeshSimpleTool )
MR_REGISTER_RIBBON_ITEM( VoxelsToMeshDualTool )
MR_REGISTER_RIBBON_ITEM( VoxelsToMeshSmartTool )
MR_REGISTER_RIBBON_ITEM( VoxelsPathTool )
MR_REGISTER_RIBBON_ITEM( VoxelsPathBuildFourTool )
MR_REGISTER_RIBBON_ITEM( BinaryOperationsTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapsLinesGcodeTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapFromMeshTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapContoursTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapIsoLinesTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapMergeTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapContourBooleanTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapFromTiffTool )
MR_REGISTER_RIBBON_ITEM( DistanceMapToTiffTool )
MR_REGISTER_RIBBON_ITEM( OffsetContoursTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesFromContoursTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesLoadMrLinesTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesSaveMrLinesTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesLoadPlyTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesSavePlyTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesLoadPtsTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesSavePtsTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesLoadSvgTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesSaveDxfTool )
MR_REGISTER_RIBBON_ITEM( ObjectLinesToContoursTool )
MR_REGISTER_RIBBON_ITEM( GcodePathParserTool )
MR_REGISTER_RIBBON_ITEM( GcodeLoadSourceTool )
MR_REGISTER_RIBBON_ITEM( GcodeWriteSourceTool )
MR_REGISTER_RIBBON_ITEM( GcodeParseFilePathsTool )
MR_REGISTER_RIBBON_ITEM( AutomationPluginApiTool )

}

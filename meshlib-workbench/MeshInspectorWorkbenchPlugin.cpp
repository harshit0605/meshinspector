#include "MRViewer/MRStatePlugin.h"
#include "MRViewer/MRSurfaceManipulationWidget.h"
#include "MRViewer/MRRibbonRegisterItem.h"
#include "MRViewer/MRShowModal.h"
#include "MRViewer/MRUIStyle.h"
#include "MRMesh/MRObjectMesh.h"
#include "MRMesh/MRObjectsAccess.h"
#include "MRMesh/MRSceneRoot.h"

#include <imgui.h>

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
        return "Selection bridge for painted, lassoed, or semantic edit masks.";
    }

    void drawDialog( ImGuiContext* ) override
    {
        if ( !ImGuiBeginWindow_( { .width = 320 * UI::scale() } ) )
            return;
        UI::transparentTextWrapped(
            "This tool is the placeholder for surface/face/brush mask capture. The web host serializes these masks into MeshInspector's interactive selection payload."
        );
        ImGui::BulletText( "Brush-paint mask capture" );
        ImGui::BulletText( "Lasso / marquee capture" );
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

}

MR_REGISTER_RIBBON_ITEM( RegionMarkTool )
MR_REGISTER_RIBBON_ITEM( ThickenBrushTool )
MR_REGISTER_RIBBON_ITEM( ScoopBrushTool )
MR_REGISTER_RIBBON_ITEM( SmoothBrushTool )
MR_REGISTER_RIBBON_ITEM( MeasureInspectTool )

}

#include <MRViewer/MRSetupViewer.h>
#include <MRViewer/MRViewer.h>
#include <MRViewer/MRRibbonMenu.h>
#include <MRViewer/MRRibbonMenuUIConfig.h>

namespace
{

class MeshInspectorWorkbenchSetup final : public MR::ViewerSetup
{
public:
    void setupConfiguration( MR::Viewer* viewer ) const override
    {
        ViewerSetup::setupConfiguration( viewer );
        if ( auto ribbon = viewer->getMenuPluginAs<MR::RibbonMenu>() )
        {
            MR::RibbonMenuUIConfig config = ribbon->getMenuUIConfig();
            config.topLayout = MR::RibbonTopPanelLayoutMode::RibbonWithTabs;
            config.drawScenePanel = true;
            config.drawToolbar = true;
            config.drawViewportTags = true;
            config.drawNotifications = true;
            config.drawSearchBar = true;
            config.helpLink = "https://meshlib.io/feature/3d-viewer/";
            ribbon->setMenuUIConfig( config );
        }
    }
};

}

int main( int argc, char** argv )
{
    MR::Viewer::LaunchParams launchParams{
        .name = "meshinspector_meshlib_workbench",
        .argc = argc,
        .argv = argv,
    };
    MR::Viewer::parseLaunchParams( launchParams );
    return MR::launchDefaultViewer( launchParams, MeshInspectorWorkbenchSetup() );
}

window.onbeforeunload = () => {
    if (typeof Module === 'undefined' || typeof Module.ccall !== 'function') {
        return;
    }
    try {
        Module.ccall('emsForceSettingsSave', 'void', [], []);
    } catch (error) {
        console.warn('MeshLib runtime settings save skipped on unload', error);
    }
};

(function () {
    let hostPayload = null;
    let runtimeReady = false;
    let loadStarted = false;

    function signalReady() {
        window.parent?.postMessage({ type: 'meshlib-workbench:ready' }, window.location.origin);
    }

    function extensionForPayload(manifest) {
        if (manifest?.normalized_mesh_url) {
            return 'ply';
        }
        if (manifest?.preview_high_url || manifest?.preview_low_url) {
            return 'glb';
        }
        return 'ply';
    }

    function meshUrlForPayload(manifest) {
        return manifest?.normalized_mesh_url || manifest?.preview_high_url || manifest?.preview_low_url || null;
    }

    async function bootFromPayload() {
        if (loadStarted || !runtimeReady || !hostPayload?.manifest) {
            return;
        }

        const manifest = hostPayload.manifest;
        const meshUrl = meshUrlForPayload(manifest);
        if (!meshUrl) {
            signalReady();
            return;
        }

        if (typeof emplace_file_in_local_FS_and_open !== 'function') {
            console.error('MeshLib runtime loaded without emplace_file_in_local_FS_and_open');
            signalReady();
            return;
        }

        loadStarted = true;
        try {
            const response = await fetch(meshUrl, { credentials: 'same-origin' });
            if (!response.ok) {
                throw new Error(`Failed to fetch runtime mesh (${response.status})`);
            }
            const bytes = new Uint8Array(await response.arrayBuffer());
            const filename = `active-version.${extensionForPayload(manifest)}`;
            emplace_file_in_local_FS_and_open(filename, bytes, function () {
                signalReady();
            });
        } catch (error) {
            console.error('Failed to load active mesh into MeshLib runtime', error);
            signalReady();
        }
    }

    const previousPostWasmLoad = window.postWasmLoad;
    window.postWasmLoad = function () {
        if (typeof previousPostWasmLoad === 'function') {
            previousPostWasmLoad();
        }
        runtimeReady = true;
        void bootFromPayload();
    };

    window.addEventListener('message', function (event) {
        if (event.origin !== window.location.origin) {
            return;
        }
        if (event.data?.type !== 'meshlib-workbench:init') {
            return;
        }
        hostPayload = event.data.payload;
        void bootFromPayload();
    });
})();

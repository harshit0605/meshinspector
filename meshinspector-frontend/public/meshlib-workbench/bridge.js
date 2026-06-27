const app = document.getElementById('app');
const title = document.getElementById('title');
const message = document.getElementById('message');

let hostPayload = null;
let runtimeManifest = null;
let runtimeFrame = null;

function renderMessage(nextTitle, nextMessage) {
  if (title) {
    title.textContent = nextTitle;
  }
  if (message) {
    message.textContent = nextMessage;
  }
}

function officialParityInventory(payload) {
  return Array.isArray(payload?.manifest?.official_parity_inventory) ? payload.manifest.official_parity_inventory : [];
}

function commandCapabilityById(payload) {
  const capabilities = Array.isArray(payload?.manifest?.command_capabilities) ? payload.manifest.command_capabilities : [];
  return new Map(capabilities.map((capability) => [capability.command_id, capability]));
}

function isProductReadyWorkbenchCapability(capability) {
  if (!capability) {
    return false;
  }
  if (capability.endpoint_url || capability.endpoint_url_key) {
    return true;
  }
  if (capability.group === 'runtime' && capability.runtime_tool_id) {
    return true;
  }
  return capability.rust_backed !== true;
}

function isOfficialParityToolEnabled(feature, payload) {
  if (!feature || feature.status === 'missing') {
    return false;
  }
  if (feature.status === 'implemented') {
    return true;
  }
  const backendCommandIds = Array.isArray(feature.backend_command_ids) ? feature.backend_command_ids : [];
  const hostedToolIds = Array.isArray(feature.hosted_tool_ids) ? feature.hosted_tool_ids : [];
  if (hostedToolIds.length > 0) {
    return true;
  }
  const capabilities = commandCapabilityById(payload);
  return backendCommandIds.some((commandId) => isProductReadyWorkbenchCapability(capabilities.get(commandId)));
}

function officialWorkbenchTools(payload) {
  return officialParityInventory(payload).map((feature) => ({
    official_feature_id: feature.official_feature_id,
    label: feature.label,
    group: feature.group,
    status: feature.status,
    enabled: isOfficialParityToolEnabled(feature, payload),
    disabled_reason: isOfficialParityToolEnabled(feature, payload) ? null : 'missing_backend_operation',
    backend_command_ids: Array.isArray(feature.backend_command_ids) ? feature.backend_command_ids : [],
    hosted_tool_ids: Array.isArray(feature.hosted_tool_ids) ? feature.hosted_tool_ids : [],
    rust_owner_modules: Array.isArray(feature.rust_owner_modules) ? feature.rust_owner_modules : [],
  }));
}

function markOfficialParityInventory(payload) {
  const inventory = officialParityInventory(payload);
  const tools = officialWorkbenchTools(payload);
  const disabledFeatureIds = tools
    .filter((tool) => !tool.enabled)
    .map((tool) => tool.official_feature_id)
    .filter(Boolean)
    .sort();
  const groups = Array.from(new Set(inventory.map((feature) => feature.group).filter(Boolean))).sort();

  document.documentElement.dataset.meshinspectorWorkbenchOfficialParityFeatureCount = String(inventory.length);
  document.documentElement.dataset.meshinspectorWorkbenchOfficialParityMissingCount = String(
    tools.filter((tool) => tool.disabled_reason === 'missing_backend_operation').length,
  );
  document.documentElement.dataset.meshinspectorWorkbenchOfficialParityGroups = groups.join(',');
  document.documentElement.dataset.meshinspectorWorkbenchDisabledFeatureIds = disabledFeatureIds.join(',');
}

function markCommandCapabilities(payload) {
  const capabilities = Array.isArray(payload?.manifest?.command_capabilities) ? payload.manifest.command_capabilities : [];
  const endpointUrls = capabilities.map((capability) => capability.endpoint_url).filter(Boolean);
  const backendEndpointCount = endpointUrls.filter((endpointUrl) => /^https?:\/\//.test(endpointUrl)).length;
  const relativeEndpointCount = endpointUrls.length - backendEndpointCount;
  const runtimeTools = Array.from(
    new Set(capabilities.map((capability) => capability.runtime_tool_id).filter(Boolean)),
  ).sort();
  document.documentElement.dataset.meshinspectorWorkbenchCommandCount = String(capabilities.length);
  document.documentElement.dataset.meshinspectorWorkbenchBackendEndpointCount = String(backendEndpointCount);
  document.documentElement.dataset.meshinspectorWorkbenchRelativeEndpointCount = String(relativeEndpointCount);
  document.documentElement.dataset.meshinspectorWorkbenchRuntimeTools = runtimeTools.join(',');
  markOfficialParityInventory(payload);
}

async function loadRuntimeManifest() {
  try {
    const response = await fetch('./runtime/manifest.json', { cache: 'no-store' });
    if (!response.ok) {
      throw new Error(`Runtime manifest unavailable (${response.status})`);
    }
    runtimeManifest = await response.json();
  } catch (error) {
    runtimeManifest = {
      status: 'missing',
      message: error instanceof Error ? error.message : 'Runtime manifest unavailable',
    };
  }
  attemptBoot();
}

function attemptBoot() {
  if (!hostPayload || !runtimeManifest) {
    return;
  }

  if (runtimeManifest.status !== 'ready') {
    renderMessage(
      'MeshLib runtime bundle missing',
      runtimeManifest.message ||
        'Build the MeshLib workbench bundle and install it into /public/meshlib-workbench/runtime to replace the classic viewer.',
    );
    return;
  }

  if (runtimeManifest.entry_html_url) {
    document.body.classList.add('runtime');
    app.classList.add('runtime');
    app.innerHTML = '';

    const frame = document.createElement('iframe');
    frame.className = 'runtime-frame';
    frame.src = runtimeManifest.entry_html_url;
    frame.title = 'MeshLib Runtime';
    runtimeFrame = frame;
    frame.addEventListener('load', () => {
      frame.contentWindow?.postMessage({ type: 'meshlib-workbench:init', payload: hostPayload }, window.location.origin);
    });
    app.appendChild(frame);
    return;
  }

  renderMessage(
    'Runtime manifest detected',
    'A runtime manifest was found, but no entry_html_url was provided. Update runtime/manifest.json to point at the compiled MeshLib viewer entry.',
  );
  window.parent?.postMessage({ type: 'meshlib-workbench:ready' }, window.location.origin);
}

window.addEventListener('message', (event) => {
  if (event.origin !== window.location.origin) {
    return;
  }
  if (
    event.source === runtimeFrame?.contentWindow &&
    [
      'meshlib-workbench:ready',
      'meshlib-workbench:commit-complete',
      'meshlib-workbench:commit-failed',
      'meshlib-workbench:selection-complete',
      'meshlib-workbench:selection-failed',
      'meshlib-workbench:brush-complete',
      'meshlib-workbench:brush-failed',
      'meshlib-workbench:measure-complete',
      'meshlib-workbench:measure-failed',
      'meshlib-workbench:host-command',
      'meshlib-workbench:command-failed',
    ].includes(event.data?.type)
  ) {
    window.parent?.postMessage(event.data, window.location.origin);
    return;
  }
  if (event.data?.type === 'meshlib-workbench:init') {
    hostPayload = event.data.payload;
    markCommandCapabilities(hostPayload);
    attemptBoot();
  }
});

window.parent?.postMessage({ type: 'meshlib-workbench:request-init' }, window.location.origin);
void loadRuntimeManifest();

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
  if (event.source === runtimeFrame?.contentWindow && event.data?.type === 'meshlib-workbench:ready') {
    window.parent?.postMessage({ type: 'meshlib-workbench:ready' }, window.location.origin);
    return;
  }
  if (event.data?.type === 'meshlib-workbench:init') {
    hostPayload = event.data.payload;
    attemptBoot();
  }
});

window.parent?.postMessage({ type: 'meshlib-workbench:request-init' }, window.location.origin);
void loadRuntimeManifest();

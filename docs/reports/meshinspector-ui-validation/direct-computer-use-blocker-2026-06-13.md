# Direct Computer Use Blocker - 2026-06-13

Scope: direct Computer Use validation of the official MeshInspector/MeshLib SDK UI in Google Chrome.

## Result

Direct Computer Use could not attach to local app windows in this Codex session. The official UI and backend validation continued through the browser harness, but the explicit direct Computer Use requirement remains blocked by the local capture/window layer.

## Evidence

- `mcp__computer_use.list_apps` reported Google Chrome, Finder, Safari, and other GUI apps as running.
- `mcp__computer_use.get_app_state({"app":"Google Chrome"})` repeatedly returned:
  - `Computer Use server error -10005: cgWindowNotFound`
- `mcp__computer_use.get_app_state({"app":"Finder"})` returned the same:
  - `Computer Use server error -10005: cgWindowNotFound`
- `mcp__computer_use.get_app_state({"app":"Codex"})` is intentionally disallowed for safety:
  - `Computer Use is not allowed to use the app 'com.openai.codex' for safety reasons.`
- Activating Google Chrome and navigating it to the official viewer via AppleScript succeeded, but Computer Use still returned `cgWindowNotFound`.
- Native macOS capture also failed:
  - `screencapture -x /tmp/meshinspector-cua-screencheck.png`
  - output: `could not create image from display`
- System Events could see the Google Chrome process, but could not read `window 1`:
  - `System Events got an error: Can't get window 1 of process "Google Chrome". Invalid index. (-1719)`

## Interpretation

The blocker is below the MeshInspector app and below Chrome navigation. It is consistent with a local macOS display/window capture or Screen Recording accessibility issue affecting Computer Use. Until that external capture layer is restored, direct Computer Use cannot provide screenshot/accessibility-tree evidence.

## Non-Blocked Evidence Already Available

- Official workbench browser harness coverage report:
  - `docs/reports/meshinspector-ui-validation/rest-command-coverage-latest.json`
  - `docs/reports/meshinspector-ui-validation/rest-command-coverage-latest.md`
- Current summary from the repeatable report:
  - Rust-backed REST commands: `80/80` passing
  - Manifest SDK operations covered by passing official UI commands: `171/171`
  - Untested commands: `0`
  - Failed commands: `0`

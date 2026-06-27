from __future__ import annotations

import re
from pathlib import Path

from api.routers import versions as versions_router


REPO_ROOT = Path(__file__).resolve().parents[2]
CASE_FILE = REPO_ROOT / "meshinspector-frontend" / "e2e" / "fixtures" / "workbenchCommandCases.ts"
E2E_SPEC_FILE = REPO_ROOT / "meshinspector-frontend" / "e2e" / "meshinspector-workbench-algorithms.spec.ts"


def _case_source() -> str:
    return CASE_FILE.read_text(encoding="utf-8")


def _command_ids_in_exported_array(name: str) -> set[str]:
    source = _case_source()
    match = re.search(rf"export const {name} = \[(.*?)\]\s+as const", source, flags=re.DOTALL)
    assert match is not None, f"Could not find exported array {name}"
    return set(re.findall(r"commandId: '([^']+)'", match.group(1)))


def _all_case_command_ids() -> set[str]:
    return set(re.findall(r"commandId: '([^']+)'", _case_source()))


def _backend_command_ids() -> set[str]:
    return {str(capability["command_id"]) for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES}


def test_workbench_e2e_bootstrap_assertions_track_current_backend_manifest_counts() -> None:
    source = E2E_SPEC_FILE.read_text(encoding="utf-8")
    expected_command_count = len(versions_router.WORKBENCH_COMMAND_CAPABILITIES)
    expected_rust_count = sum(
        1
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability.get("rust_backed") is True
    )

    assert f"toHaveLength({expected_command_count})" in source
    assert f"toBe({expected_command_count})" in source
    assert f"toHaveLength({expected_rust_count})" in source


def test_every_backend_workbench_command_is_classified_by_the_e2e_matrix() -> None:
    assert len(versions_router.WORKBENCH_COMMAND_CAPABILITIES) == 90
    assert _all_case_command_ids() == _backend_command_ids()


def test_every_customer_runnable_workbench_command_has_an_executable_e2e_case() -> None:
    endpoint_backed = {
        str(capability["command_id"])
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability.get("endpoint_url_key") is not None
    }
    non_endpoint_ui_workflows = {"upload-new", "wireframe"}
    executable = _command_ids_in_exported_array("customerRunnableCommandCases")

    assert endpoint_backed | non_endpoint_ui_workflows <= executable


def test_every_non_endpoint_rust_command_is_classified_as_sdk_only_gap() -> None:
    sdk_only_gaps = _command_ids_in_exported_array("sdkOnlyGapCases")
    unendpointed_rust = {
        str(capability["command_id"])
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability.get("rust_backed") is True
        and capability.get("endpoint_url_key") is None
        and not str(capability["command_id"]).startswith("runtime-")
    }

    assert unendpointed_rust == sdk_only_gaps


def test_sdk_only_gap_cases_are_not_accidentally_customer_ready() -> None:
    capability_by_id = {
        str(capability["command_id"]): capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }
    for command_id in _command_ids_in_exported_array("sdkOnlyGapCases"):
        capability = capability_by_id[command_id]
        assert capability.get("rust_backed") is True
        assert capability.get("endpoint_url_key") is None

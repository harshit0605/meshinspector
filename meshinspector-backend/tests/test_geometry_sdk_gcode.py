from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.gcode import (
    GcodeMachineSettings,
    GcodePathDocument,
    load_gcode_source,
    parse_gcode_file_paths,
    parse_gcode_paths,
    write_gcode_source,
)


GCODE_SAMPLE = """
; header retained as a non-empty MeshLib frame
G90
G0 X0 Y0 Z1 F3000
G1 X1 Y0 F1200
Y1 ; modal G1 movement
G91
X0.5 (relative modal movement)
G20
X1
G21
"""


def test_parse_gcode_paths_matches_meshlib_linear_processor_semantics() -> None:
    parsed = parse_gcode_paths(GCODE_SAMPLE)

    assert isinstance(parsed, GcodePathDocument)
    assert parsed.frame_count == 10
    assert parsed.command_count == 16
    assert parsed.segment_count == 5
    assert parsed.source_frame_indices.tolist() == [2, 3, 4, 6, 8]
    assert parsed.idle.tolist() == [True, False, False, False, False]
    np.testing.assert_allclose(parsed.segments[0], [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0]])
    np.testing.assert_allclose(parsed.segments[1], [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0]])
    np.testing.assert_allclose(parsed.segments[2, 1], [1.0, 1.0, 1.0])
    np.testing.assert_allclose(parsed.segments[3, 1], [1.5, 1.0, 1.0])
    np.testing.assert_allclose(parsed.segments[4, 1], [26.9, 1.0, 1.0])
    assert parsed.feedrates[1] == pytest.approx(1200.0)
    assert parsed.feedrates[4] == pytest.approx(1200.0)
    assert parsed.max_feedrate == pytest.approx(1200.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_strtof_command_narrowing() -> None:
    parsed = parse_gcode_paths("G90\nG1 X0.123456789 Y0.333333333 Z0.100000001 F1234.56789\n")

    expected_end = np.asarray([0.123456789, 0.333333333, 0.100000001], dtype=np.float32)
    np.testing.assert_array_equal(parsed.segments[0, 1], expected_end.astype(np.float64))
    assert parsed.feedrates[0] == float(np.float32(1234.56789))
    assert parsed.max_feedrate == float(np.float32(1234.56789))
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_strtof_special_float_tokens() -> None:
    parsed = parse_gcode_paths("G90\nG1 Xnan F600\n")

    assert parsed.frame_count == 2
    assert parsed.command_count == 4
    assert parsed.segment_count == 1
    assert np.isnan(parsed.segments[0, 1, 0])
    assert parsed.feedrates.tolist() == [600.0]
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_strtof_hex_float_tokens() -> None:
    parsed = parse_gcode_paths("G90\nG1 X0x1p+2 F600\n")

    assert parsed.frame_count == 2
    assert parsed.command_count == 4
    assert parsed.segment_count == 1
    np.testing.assert_array_equal(parsed.segments[0, 1], [4.0, 0.0, 0.0])
    assert parsed.feedrates.tolist() == [600.0]
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_strtof_leading_whitespace() -> None:
    parsed = parse_gcode_paths("G90\nG1 X 2 F600\n")

    assert parsed.frame_count == 2
    assert parsed.command_count == 4
    assert parsed.segment_count == 1
    np.testing.assert_array_equal(parsed.segments[0, 1], [2.0, 0.0, 0.0])
    assert parsed.feedrates.tolist() == [600.0]
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_gcode_source_file_workflow_matches_meshlib_supported_formats(tmp_path) -> None:
    path = tmp_path / "program.NC"
    path.write_text("\n; retained comment frame\nG90\n\nG0 X0 Y0 Z0 F3000\nG1 X1 Y2 Z3 F600\n")

    frames = load_gcode_source(path)
    parsed = parse_gcode_file_paths(path)

    assert frames == [
        "; retained comment frame",
        "G90",
        "G0 X0 Y0 Z0 F3000",
        "G1 X1 Y2 Z3 F600",
    ]
    assert parsed.frame_count == 4
    assert parsed.command_count == 11
    assert parsed.segment_count == 2
    assert parsed.source_frame_indices.tolist() == [2, 3]
    assert parsed.idle.tolist() == [True, False]
    assert parsed.feedrates.tolist() == [10_000.0, 600.0]


def test_gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns(tmp_path) -> None:
    path = tmp_path / "program.gcode"
    path.write_text("G90\r\nG1 X1 Y2\r\n\nG1 X3\r\n")

    frames = load_gcode_source(path)
    parsed = parse_gcode_file_paths(path)

    assert frames == ["G90\r", "G1 X1 Y2\r", "G1 X3\r"]
    assert parsed.frame_count == 3
    assert parsed.command_count == 6
    assert parsed.source_frame_indices.tolist() == [1, 2]
    np.testing.assert_allclose(parsed.segments[0, 1], [1.0, 2.0, 0.0])
    np.testing.assert_allclose(parsed.segments[1, 1], [3.0, 2.0, 0.0])


def test_gcode_source_file_export_roundtrips_meshlib_object_gcode_source_frames(tmp_path) -> None:
    path = tmp_path / "program.gcode"
    frames = ["G90", "G0 X0 Y0 Z0", "G1 X1 Y0 F500"]

    output_path = write_gcode_source(frames, path)

    assert output_path == path
    assert load_gcode_source(path) == frames


def test_geometry_sdk_exposes_gcode_path_parser() -> None:
    parsed = GeometrySDK().parse_gcode_paths(GCODE_SAMPLE)

    assert parsed.segment_count == 5
    np.testing.assert_allclose(parsed.segments[-1, -1], [26.9, 1.0, 1.0])


def test_parse_gcode_paths_matches_meshlib_center_offset_arc_sampling() -> None:
    parsed = parse_gcode_paths("G90\nG0 X1 Y0 Z0\nG3 X0 Y1 I-1 J0 F600\n")

    assert parsed.frame_count == 3
    assert parsed.command_count == 11
    assert parsed.segment_count == 16
    assert parsed.source_frame_indices.tolist() == [1] + [2] * 15
    assert parsed.idle.tolist() == [True] + [False] * 15
    np.testing.assert_allclose(parsed.segments[1, 0], [1.0, 0.0, 0.0])
    np.testing.assert_allclose(
        parsed.segments[1, 1],
        [np.cos(np.pi / 30.0), np.sin(np.pi / 30.0), 0.0],
        atol=1e-9,
    )
    np.testing.assert_allclose(parsed.segments[-1, 1], [0.0, 1.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(parsed.feedrates[1:], np.full(15, 600.0))
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_arc_radius_mismatch_warning_format() -> None:
    parsed = parse_gcode_paths("G90\nG0 X1 Y0 Z0\nG3 X0 Y2 I-1 J0 F600\n")

    assert parsed.frame_count == 3
    assert parsed.command_count == 11
    assert parsed.source_frame_indices[0] == 1
    assert parsed.source_frame_indices[1:].tolist() == [2] * (parsed.segment_count - 1)
    assert parsed.warnings == ["frame 2: Begin and end radius are different: diff = 1.732051"]


def test_parse_gcode_paths_matches_meshlib_radius_only_arc_no_motion_feedrate_contract() -> None:
    parsed = parse_gcode_paths("G90\nG0 X1 Y0 Z0\nG2 R1 F600\n")

    assert parsed.frame_count == 3
    assert parsed.command_count == 8
    assert parsed.segment_count == 1
    assert parsed.source_frame_indices.tolist() == [1]
    assert parsed.idle.tolist() == [True]
    np.testing.assert_allclose(parsed.segments[0, 1], [1.0, 0.0, 0.0])
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_feedrate_only_frame_without_segments() -> None:
    parsed = parse_gcode_paths("G90\nG1 F600\nG0 X1\n")

    assert parsed.frame_count == 3
    assert parsed.command_count == 5
    assert parsed.segment_count == 1
    assert parsed.source_frame_indices.tolist() == [2]
    assert parsed.idle.tolist() == [True]
    np.testing.assert_allclose(parsed.segments[0, 1], [1.0, 0.0, 0.0])
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_g18_g19_work_plane_mapping() -> None:
    zx = parse_gcode_paths("G90\nG18\nG0 X1 Y0 Z0\nG2 X0 Y0 Z1 I-1 J0 K0 F600\n")

    assert zx.segment_count == 16
    assert zx.source_frame_indices.tolist() == [2] + [3] * 15
    np.testing.assert_allclose(zx.segments[1, 0], [1.0, 0.0, 0.0])
    np.testing.assert_allclose(
        zx.segments[1, 1],
        [np.cos(np.pi / 30.0), 0.0, np.sin(np.pi / 30.0)],
        atol=1e-9,
    )
    np.testing.assert_allclose(zx.segments[-1, 1], [0.0, 0.0, 1.0], atol=1e-9)
    assert zx.warnings == []

    yz = parse_gcode_paths("G90\nG19\nG0 X0 Y1 Z0\nG3 X0 Y0 Z1 I0 J-1 K0 F700\n")

    assert yz.segment_count == 16
    assert yz.source_frame_indices.tolist() == [2] + [3] * 15
    np.testing.assert_allclose(yz.segments[1, 0], [0.0, 1.0, 0.0])
    np.testing.assert_allclose(
        yz.segments[1, 1],
        [0.0, np.cos(np.pi / 30.0), np.sin(np.pi / 30.0)],
        atol=1e-9,
    )
    np.testing.assert_allclose(yz.segments[-1, 1], [0.0, 0.0, 1.0], atol=1e-9)
    np.testing.assert_allclose(yz.feedrates[1:], np.full(15, 700.0))
    assert yz.max_feedrate == pytest.approx(700.0)
    assert yz.warnings == []


def test_parse_gcode_paths_matches_meshlib_g51_scaling_contract() -> None:
    parsed = parse_gcode_paths(
        "G90\n"
        "G51 X2 Y3 Z4\n"
        "G0 X1 Y1 Z1\n"
        "G51 X0 Y5\n"
        "G1 X2 Y2 Z2 F500\n"
        "G50\n"
        "G1 X2 Y2 Z2\n"
    )

    assert parsed.frame_count == 7
    assert parsed.command_count == 22
    assert parsed.segment_count == 3
    assert parsed.source_frame_indices.tolist() == [2, 4, 6]
    assert parsed.idle.tolist() == [True, False, False]
    np.testing.assert_allclose(parsed.segments[0], [[0.0, 0.0, 0.0], [2.0, 3.0, 4.0]])
    np.testing.assert_allclose(parsed.segments[1], [[2.0, 3.0, 4.0], [4.0, 10.0, 8.0]])
    np.testing.assert_allclose(parsed.segments[2], [[4.0, 10.0, 8.0], [2.0, 2.0, 2.0]])
    np.testing.assert_allclose(parsed.feedrates[1:], [500.0, 500.0])
    assert parsed.max_feedrate == pytest.approx(500.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_default_c_axis_rotation_sampling() -> None:
    parsed = parse_gcode_paths("G90\nG0 X1 Y0 Z0\nG1 C90 F600\nG1 X2 Y0 C180 F700\n")

    assert parsed.frame_count == 4
    assert parsed.command_count == 13
    assert parsed.segment_count == 41
    assert parsed.source_frame_indices.tolist() == [1] + [2] * 20 + [3] * 20
    assert parsed.idle.tolist() == [True] + [False] * 40
    np.testing.assert_allclose(parsed.segments[0], [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])
    np.testing.assert_allclose(
        parsed.segments[1, 1],
        [np.cos(np.deg2rad(4.5)), np.sin(np.deg2rad(4.5)), 0.0],
        atol=1e-9,
    )
    np.testing.assert_allclose(parsed.segments[20, 1], [0.0, 1.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(parsed.segments[21, 0], [0.0, 1.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(
        parsed.segments[30, 1],
        [1.5 * np.cos(np.deg2rad(135.0)), 1.5 * np.sin(np.deg2rad(135.0)), 0.0],
        atol=1e-9,
    )
    np.testing.assert_allclose(parsed.segments[40, 1], [-2.0, 0.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(parsed.feedrates[1:21], np.full(20, 600.0))
    np.testing.assert_allclose(parsed.feedrates[21:], np.full(20, 700.0))
    assert parsed.max_feedrate == pytest.approx(700.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_exports_meshlib_default_tool_directions() -> None:
    parsed = parse_gcode_paths("G90\nG0 X0 Y0 Z1\nG1 A90 F600\n")

    assert parsed.frame_count == 3
    assert parsed.command_count == 8
    assert parsed.segment_count == 21
    np.testing.assert_allclose(parsed.tool_directions[0], [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]])
    np.testing.assert_allclose(parsed.tool_directions[1, 0], [0.0, 0.0, 1.0])
    np.testing.assert_allclose(
        parsed.tool_directions[1, 1],
        [0.0, np.sin(np.deg2rad(4.5)), np.cos(np.deg2rad(4.5))],
        atol=1e-9,
    )
    np.testing.assert_allclose(parsed.tool_directions[20, 1], [0.0, 1.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(parsed.feedrates[1:], np.full(20, 600.0))
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_accepts_meshlib_cnc_home_and_idle_settings() -> None:
    settings = GcodeMachineSettings(home_position=(2.0, 3.0, 4.0), feedrate_idle=1234.0)

    parsed = parse_gcode_paths("G90\nG0 X1 Y0 Z0\nG28\n", machine_settings=settings)

    assert parsed.frame_count == 3
    assert parsed.command_count == 6
    assert parsed.segment_count == 2
    assert parsed.source_frame_indices.tolist() == [1, 2]
    assert parsed.idle.tolist() == [True, True]
    np.testing.assert_allclose(parsed.segments[0], [[2.0, 3.0, 4.0], [1.0, 0.0, 0.0]])
    np.testing.assert_allclose(parsed.segments[1], [[1.0, 0.0, 0.0], [2.0, 3.0, 4.0]])
    np.testing.assert_allclose(parsed.feedrates, np.full(2, 1234.0))
    assert parsed.max_feedrate == pytest.approx(0.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_zero_idle_feedrate_post_pass() -> None:
    settings = GcodeMachineSettings(feedrate_idle=0.0)

    parsed = parse_gcode_paths("G90\nG0 X1\nG1 X2 F600\n", machine_settings=settings)

    assert parsed.frame_count == 3
    assert parsed.command_count == 6
    assert parsed.segment_count == 2
    assert parsed.source_frame_indices.tolist() == [1, 2]
    assert parsed.idle.tolist() == [True, False]
    assert parsed.feedrates.tolist() == [600.0, 600.0]
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_matches_meshlib_g28_at_home_zero_length_idle_action() -> None:
    settings = GcodeMachineSettings(feedrate_idle=1234.0)

    parsed = parse_gcode_paths("G90\nG28\n", machine_settings=settings)

    assert parsed.frame_count == 2
    assert parsed.command_count == 2
    assert parsed.segment_count == 1
    assert parsed.source_frame_indices.tolist() == [1]
    assert parsed.idle.tolist() == [True]
    np.testing.assert_allclose(parsed.segments[0], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]])
    assert parsed.feedrates.tolist() == [1234.0]
    assert parsed.max_feedrate == pytest.approx(0.0)
    assert parsed.warnings == []


def test_parse_gcode_paths_accepts_meshlib_cnc_rotation_axes_and_order_settings() -> None:
    axis_settings = GcodeMachineSettings(
        rotation_axes=((0.0, 0.0, 2.0), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0)),
        rotation_order=("A",),
    )

    axis_parsed = parse_gcode_paths(
        "G90\nG0 X1 Y0 Z0\nG1 A90 F600\n",
        machine_settings=axis_settings,
    )

    assert axis_parsed.segment_count == 21
    np.testing.assert_allclose(axis_parsed.segments[20, 1], [0.0, 1.0, 0.0], atol=1e-9)
    np.testing.assert_allclose(axis_parsed.tool_directions[20, 1], [0.0, 0.0, 1.0])
    assert axis_parsed.max_feedrate == pytest.approx(600.0)

    order_settings = GcodeMachineSettings(rotation_order=("C", "A", "C"))

    order_parsed = parse_gcode_paths(
        "G90\nG0 X0 Y1 Z0\nG1 A90 C90 F700\n",
        machine_settings=order_settings,
    )

    assert order_parsed.segment_count == 21
    np.testing.assert_allclose(order_parsed.segments[20, 1], [-1.0, 0.0, 0.0], atol=1e-9)
    assert order_parsed.max_feedrate == pytest.approx(700.0)
    assert order_parsed.warnings == []


def test_parse_gcode_paths_accepts_meshlib_cnc_rotation_limit_settings() -> None:
    settings = GcodeMachineSettings(rotation_limits=((-45.0, 45.0), None, None))

    parsed = parse_gcode_paths(
        "G90\nG0 X0 Y0 Z1\nG1 A90 F600\n",
        machine_settings=settings,
    )

    assert parsed.segment_count == 21
    assert parsed.max_feedrate == pytest.approx(600.0)
    assert parsed.warnings == ["frame 2: Error input angle: Going beyond the limits."]

    ignored_invalid_limits = GcodeMachineSettings(rotation_limits=((45.0, -45.0), None, None))

    no_warning = parse_gcode_paths(
        "G90\nG0 X0 Y0 Z1\nG1 A90 F600\n",
        machine_settings=ignored_invalid_limits,
    )

    assert no_warning.warnings == []


def test_parse_gcode_paths_clamps_meshlib_cnc_rotation_limit_settings() -> None:
    settings = GcodeMachineSettings(rotation_limits=((-240.0, 240.0), (-240.0, 0.0), None))

    inside_clamped_a = parse_gcode_paths(
        "G90\nG0 X0 Y0 Z1\nG1 A180 F600\n",
        machine_settings=settings,
    )
    assert inside_clamped_a.warnings == []

    outside_clamped_b = parse_gcode_paths(
        "G90\nG0 X0 Y0 Z1\nG1 B90 F700\n",
        machine_settings=settings,
    )
    assert outside_clamped_b.warnings == ["frame 2: Error input angle: Going beyond the limits."]


def test_gcode_machine_settings_exports_meshlib_cnc_json_contract() -> None:
    settings = GcodeMachineSettings(
        home_position=(1.0, 2.0, 3.0),
        feedrate_idle=1234.0,
        rotation_axes=((0.0, 0.0, 2.0), (0.0, -3.0, 0.0), (4.0, 0.0, 0.0)),
        rotation_order=("C", "A", "C", "B"),
        rotation_limits=((-240.0, 240.0), None, (20.0, 10.0)),
    )

    meshlib_json = settings.to_meshlib_json()

    assert meshlib_json == {
        "Axes Order": "CAB",
        "Axis A": {
            "Direction": {"x": 0.0, "y": 0.0, "z": 1.0},
            "Limits": {"x": -180.0, "y": 180.0},
        },
        "Axis B": {"Direction": {"x": 0.0, "y": -1.0, "z": 0.0}, "Limits": False},
        "Axis C": {"Direction": {"x": 1.0, "y": 0.0, "z": 0.0}, "Limits": False},
        "Feedrate Idle": 1234.0,
        "Home Position": {"x": 1.0, "y": 2.0, "z": 3.0},
    }


def test_gcode_machine_settings_imports_meshlib_cnc_json_contract() -> None:
    settings = GcodeMachineSettings.from_meshlib_json(
        {
            "Axes Order": "CA",
            "Axis C": {"Direction": {"x": 4.0, "y": 0.0, "z": 0.0}, "Limits": False},
            "Axis A": {
                "Direction": {"x": 0.0, "y": 0.0, "z": 2.0},
                "Limits": {"x": -240.0, "y": 240.0},
            },
            "Feedrate Idle": 1234.0,
            "Home Position": {"x": 1.0, "y": 2.0, "z": 3.0},
        }
    )

    assert settings.to_payload() == {
        "home_position": [1.0, 2.0, 3.0],
        "feedrate_idle": 1234.0,
        "rotation_axes": [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]],
        "rotation_order": [2, 0],
        "rotation_limits": [[-180.0, 180.0], None, None],
    }

    with pytest.raises(ValueError, match="Axes Order"):
        GcodeMachineSettings.from_meshlib_json(
            {
                "Axes Order": "AA",
                "Axis A": {"Direction": {"x": 1.0, "y": 0.0, "z": 0.0}, "Limits": False},
                "Feedrate Idle": 1234.0,
                "Home Position": {"x": 1.0, "y": 2.0, "z": 3.0},
            }
        )

    with pytest.raises(ValueError, match="Feedrate Idle"):
        GcodeMachineSettings.from_meshlib_json(
            {
                "Axes Order": "A",
                "Axis A": {"Direction": {"x": 1.0, "y": 0.0, "z": 0.0}, "Limits": False},
                "Feedrate Idle": False,
                "Home Position": {"x": 1.0, "y": 2.0, "z": 3.0},
            }
        )


def test_gcode_machine_settings_imports_meshlib_string_vector_json_contract() -> None:
    settings = GcodeMachineSettings.from_meshlib_json(
        {
            "Axes Order": "AC",
            "Axis A": {"Direction": "0 0 2", "Limits": "-240 240"},
            "Axis C": {"Direction": "4 0 0", "Limits": False},
            "Feedrate Idle": 1234.0,
            "Home Position": "1 2 3",
        }
    )

    assert settings.to_payload() == {
        "home_position": [1.0, 2.0, 3.0],
        "feedrate_idle": 1234.0,
        "rotation_axes": [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]],
        "rotation_order": [0, 2],
        "rotation_limits": [[-180.0, 180.0], None, None],
    }


def test_gcode_machine_settings_imports_meshlib_hex_float_string_vectors() -> None:
    settings = GcodeMachineSettings.from_meshlib_json(
        {
            "Axes Order": "A",
            "Axis A": {"Direction": "0x0p+0 0x0p+0 0x1p+1", "Limits": "-0x1p+8 0x1p+8"},
            "Feedrate Idle": 1234.0,
            "Home Position": "0x1p+0 0x1p+1 0x1.8p+1",
        }
    )

    assert settings.to_payload() == {
        "home_position": [1.0, 2.0, 3.0],
        "feedrate_idle": 1234.0,
        "rotation_axes": [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
        "rotation_order": [0],
        "rotation_limits": [[-180.0, 180.0], None, None],
    }


def test_gcode_machine_settings_imports_meshlib_numeric_prefix_string_vectors() -> None:
    settings = GcodeMachineSettings.from_meshlib_json(
        {
            "Axes Order": "A",
            "Axis A": {"Direction": "0 0 2suffix", "Limits": "-240 240deg"},
            "Feedrate Idle": 1234.0,
            "Home Position": "1 2 3mm",
        }
    )

    assert settings.to_payload() == {
        "home_position": [1.0, 2.0, 3.0],
        "feedrate_idle": 1234.0,
        "rotation_axes": [[0.0, 0.0, 1.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
        "rotation_order": [0],
        "rotation_limits": [[-180.0, 180.0], None, None],
    }


def test_gcode_machine_settings_imports_meshlib_partial_direction_string_vector() -> None:
    settings = GcodeMachineSettings.from_meshlib_json(
        {
            "Axes Order": "A",
            "Axis A": {"Direction": "1 2", "Limits": False},
            "Feedrate Idle": 1234.0,
            "Home Position": "1 2 3",
        }
    )

    payload = settings.to_payload()
    np.testing.assert_allclose(
        payload["rotation_axes"][0],
        [1.0 / np.sqrt(5.0), 2.0 / np.sqrt(5.0), 0.0],
    )
    assert payload["rotation_order"] == [0]
    assert payload["rotation_limits"] == [None, None, None]

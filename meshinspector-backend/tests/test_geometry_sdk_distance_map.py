from __future__ import annotations

import pytest
import numpy as np
from PIL import Image

from geometry_sdk import GeometrySDK
from geometry_sdk.types import MeshDocument
from geometry_sdk.distance_map import (
    DistanceMapDocument,
    IsoLineSegmentsDocument,
    ObjectLinesDocument,
    distance_map_contour_boolean,
    distance_map_from_contours,
    distance_map_from_mesh,
    distance_map_from_tiff,
    distance_map_to_tiff,
    distance_map_merge,
    distance_map_to_iso_segments,
    offset_contours,
    offset_contours_with_origins,
    object_lines_from_contours,
    object_lines_from_mrlines,
    object_lines_from_ply,
    object_lines_from_pts,
    object_lines_to_dxf,
    object_lines_to_contours,
    object_lines_to_mrlines,
    object_lines_to_ply,
    object_lines_to_pts,
)


SQUARE_CONTOUR = [
    [
        (0.0, 0.0),
        (2.0, 0.0),
        (2.0, 2.0),
        (0.0, 2.0),
        (0.0, 0.0),
    ]
]


def test_distance_map_from_contours_matches_meshlib_pixel_center_signed_contract() -> None:
    distance_map = distance_map_from_contours(
        SQUARE_CONTOUR,
        width=3,
        height=3,
        origin=(0.0, 0.0),
        pixel_size=1.0,
        signed=True,
    )

    assert isinstance(distance_map, DistanceMapDocument)
    assert distance_map.width == 3
    assert distance_map.height == 3
    assert distance_map.valid_count == 9
    assert distance_map.values.shape == (3, 3)
    assert distance_map.values[0, 0] == pytest.approx(-0.5)
    assert distance_map.values[1, 1] == pytest.approx(-0.5)
    assert distance_map.values[0, 2] == pytest.approx(0.5)
    assert distance_map.min_value < 0.0
    assert distance_map.max_value > 0.0


def test_distance_map_from_open_contour_remains_unsigned() -> None:
    distance_map = distance_map_from_contours(
        [[(0.0, 0.0), (2.0, 0.0)]],
        width=3,
        height=2,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        signed=True,
    )

    assert distance_map.valid_count == 6
    assert distance_map.values.min() >= 0.0
    assert distance_map.values[0, 0] == pytest.approx(0.5)
    assert distance_map.values[0, 1] == pytest.approx(0.5)
    assert distance_map.values[0, 2] == pytest.approx(2**-0.5)


def test_object_lines_from_contours_matches_meshlib_scene_json_contract() -> None:
    lines = object_lines_from_contours(
        [
            [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 0.0, 0.0)],
            [(2.0, 0.0, 0.0), (3.0, 0.0, 0.0), (3.0, 1.0, 0.0)],
        ],
        line_width=2.5,
        show_points=1,
        smooth_connections=0,
    )

    assert isinstance(lines, ObjectLinesDocument)
    np.testing.assert_allclose(
        lines.points,
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
        ],
    )
    assert lines.lines.tolist() == [[0, 1], [1, 2], [2, 0], [3, 4], [4, 5]]
    meshlib_json = lines.to_meshlib_json()
    assert meshlib_json["Type"] == ["LinesHolder", "ObjectLines"]
    assert meshlib_json["ShowPoints"] == 1
    assert meshlib_json["SmoothConnections"] == 0
    assert meshlib_json["ColoringType"] == "Solid"
    assert meshlib_json["LineWidth"] == pytest.approx(2.5)
    assert meshlib_json["Polyline"]["Points"] == lines.points.tolist()
    assert meshlib_json["Polyline"]["Lines"] == [0, 1, 1, 2, 2, 0, 3, 4, 4, 5]


def test_object_lines_to_contours_roundtrips_closed_and_open_meshlib_components() -> None:
    lines = object_lines_from_contours(
        [
            [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)],
            [(2.0, 0.0), (3.0, 0.0), (3.0, 1.0)],
        ],
    )

    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 0.0, 0.0)],
        [(2.0, 0.0, 0.0), (3.0, 0.0, 0.0), (3.0, 1.0, 0.0)],
    ]
    assert GeometrySDK().object_lines_to_contours(lines) == object_lines_to_contours(lines)


def test_offset_contours_matches_meshlib_closed_clockwise_round_corner_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, -0.25, 0.0],
            [-0.077254, -0.237764, 0.0],
            [-0.146946, -0.202254, 0.0],
            [-0.202254, -0.146946, 0.0],
            [-0.237764, -0.077254, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.237764, 2.077254, 0.0],
            [-0.202254, 2.146946, 0.0],
            [-0.146946, 2.202254, 0.0],
            [-0.077254, 2.237764, 0.0],
            [0.0, 2.25, 0.0],
            [2.0, 2.25, 0.0],
            [2.077254, 2.237764, 0.0],
            [2.146946, 2.202254, 0.0],
            [2.202254, 2.146946, 0.0],
            [2.237764, 2.077254, 0.0],
            [2.25, 2.0, 0.0],
            [2.25, 0.0, 0.0],
            [2.237764, -0.077254, 0.0],
            [2.202254, -0.146946, 0.0],
            [2.146946, -0.202254, 0.0],
            [2.077254, -0.237764, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
    ) == result


def test_offset_contours_with_origins_matches_meshlib_positive_round_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    assert [(origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) for origin in result["origins"][0]] == [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 0),
    ]
    assert all(not origin["is_intersection"] for origin in result["origins"][0])
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
    ) == result


def test_offset_contours_with_origins_matches_meshlib_positive_fixed_self_overlap_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 3.0), (1.0, 3.0), (1.0, 1.0), (3.0, 1.0), (3.0, 0.0), (0.0, 0.0)]],
        offset=0.20,
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 32
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, -0.2, 0.0],
            [-0.061803, -0.190211, 0.0],
            [-0.117557, -0.161803, 0.0],
            [-0.161803, -0.117557, 0.0],
            [-0.190211, -0.061803, 0.0],
            [-0.2, 0.0, 0.0],
            [-0.2, 3.0, 0.0],
            [-0.190211, 3.061803, 0.0],
            [-0.161803, 3.117557, 0.0],
            [-0.117557, 3.161803, 0.0],
            [-0.061803, 3.190211, 0.0],
            [0.0, 3.2, 0.0],
            [1.0, 3.2, 0.0],
            [1.061803, 3.190211, 0.0],
            [1.117557, 3.161803, 0.0],
            [1.161803, 3.117557, 0.0],
            [1.190211, 3.061803, 0.0],
            [1.2, 3.0, 0.0],
            [1.2, 1.2, 0.0],
            [3.0, 1.2, 0.0],
            [3.061803, 1.190211, 0.0],
            [3.117557, 1.161803, 0.0],
            [3.161803, 1.117557, 0.0],
            [3.190211, 1.061804, 0.0],
            [3.2, 1.0, 0.0],
            [3.2, 0.0, 0.0],
            [3.190211, -0.061803, 0.0],
            [3.161803, -0.117557, 0.0],
            [3.117557, -0.161803, 0.0],
            [3.061803, -0.190211, 0.0],
            [3.0, -0.2, 0.0],
            [0.0, -0.2, 0.0],
        ],
        atol=1e-6,
    )
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected_lorg_vertices = [
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        1,
        1,
        1,
        1,
        1,
        2,
        2,
        2,
        2,
        2,
        2,
        3,
        4,
        4,
        4,
        4,
        4,
        4,
        5,
        5,
        5,
        5,
        5,
        5,
        0,
    ]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 18:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 2)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 3)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 4)
            assert origin["l_ratio"] == pytest.approx(0.1)
            assert origin["u_ratio"] == pytest.approx(0.1)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 3.0), (1.0, 3.0), (1.0, 1.0), (3.0, 1.0), (3.0, 0.0), (0.0, 0.0)]],
        offset=0.20,
    ) == result


def test_offset_contours_with_origins_matches_meshlib_positive_variable_self_overlap_index_map_contract() -> None:
    contour = [
        (0.0, 0.0),
        (0.0, 3.0),
        (1.0, 3.0),
        (1.0, 1.0),
        (3.0, 1.0),
        (3.0, 0.0),
        (0.0, 0.0),
    ]
    fixed = offset_contours_with_origins([contour], offset=0.20)
    variable = offset_contours_with_origins(
        [contour],
        offsets=[[0.20, 0.20, 0.20, 0.20, 0.20, 0.20, 0.20]],
    )

    assert variable == fixed
    assert GeometrySDK().offset_contours_with_origins(
        [contour],
        offsets=[[0.20, 0.20, 0.20, 0.20, 0.20, 0.20, 0.20]],
    ) == variable


def test_offset_contours_with_origins_matches_meshlib_positive_variable_unequal_self_overlap_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 3.0), (1.0, 3.0), (1.0, 1.0), (3.0, 1.0), (3.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.24, 0.18, 0.28, 0.22, 0.26, 0.20]],
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 32
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, -0.2, 0.0],
            [-0.078197, -0.192447, 0.0],
            [-0.134611, -0.1715, 0.0],
            [-0.171927, -0.13433, 0.0],
            [-0.192829, -0.078107, 0.0],
            [-0.2, 0.0, 0.0],
            [-0.24, 3.0, 0.0],
            [-0.233211, 3.095109, 0.0],
            [-0.208304, 3.165338, 0.0],
            [-0.162792, 3.212013, 0.0],
            [-0.094186, 3.236458, 0.0],
            [0.0, 3.24, 0.0],
            [1.0, 3.18, 0.0],
            [1.06982, 3.171119, 0.0],
            [1.119634, 3.151979, 0.0],
            [1.152538, 3.119279, 0.0],
            [1.171628, 3.069719, 0.0],
            [1.18, 3.0, 0.0],
            [1.2664, 1.272008, 0.0],
            [3.0, 1.22, 0.0],
            [3.085579, 1.211048, 0.0],
            [3.146789, 1.187905, 0.0],
            [3.18721, 1.147238, 0.0],
            [3.210421, 1.085714, 0.0],
            [3.22, 1.0, 0.0],
            [3.26, 0.0, 0.0],
            [3.254669, -0.102235, 0.0],
            [3.227996, -0.176816, 0.0],
            [3.177988, -0.22628, 0.0],
            [3.102653, -0.253162, 0.0],
            [3.0, -0.26, 0.0],
            [0.0, -0.2, 0.0],
        ],
        atol=1e-6,
    )
    origin = result["origins"][0][18]
    assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, 3)
    assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 4)
    assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
    assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 3)
    assert origin["l_ratio"] == pytest.approx(0.1332, abs=1e-6)
    assert origin["u_ratio"] == pytest.approx(0.863996, abs=1e-6)
    assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 3.0), (1.0, 3.0), (1.0, 1.0), (3.0, 1.0), (3.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.24, 0.18, 0.28, 0.22, 0.26, 0.20]],
    ) == result


def test_offset_contours_with_origins_matches_meshlib_negative_intersection_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected = [
        ((0, 0), (0, 3), (0, 1), (0, 0), 0.125, 0.875),
        ((0, 1), (0, 2), (0, 1), (0, 0), 0.125, 0.125),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.875, 0.875),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.125, 0.875),
        ((0, 0), (0, 3), (0, 1), (0, 0), 0.125, 0.875),
    ]
    for origin, expected_origin in zip(result["origins"][0], expected):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == expected_origin[0]
        assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == expected_origin[1]
        assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == expected_origin[2]
        assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == expected_origin[3]
        assert origin["l_ratio"] == pytest.approx(expected_origin[4])
        assert origin["u_ratio"] == pytest.approx(expected_origin[5])
        assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
    ) == result


def test_offset_contours_with_origins_matches_meshlib_zero_offset_identity_index_map_contract() -> None:
    contour = [
        (0.0, 0.0, 1.0),
        (0.0, 2.0, 2.0),
        (2.0, 2.0, 3.0),
        (2.0, 0.0, 4.0),
        (0.0, 0.0, 1.0),
    ]

    result = offset_contours_with_origins([contour], offset=0.0)

    assert result["contours"] == [
        [
            (0.0, 0.0, 2.0),
            (0.0, 2.0, 2.0),
            (2.0, 2.0, 3.0),
            (2.0, 0.0, 3.0),
            (0.0, 0.0, 2.0),
        ]
    ]
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(contour)
    assert [(origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) for origin in result["origins"][0]] == [
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 0),
    ]
    assert all(not origin["is_intersection"] for origin in result["origins"][0])
    assert GeometrySDK().offset_contours_with_origins([contour], offset=0.0) == result


def test_offset_contours_with_origins_matches_meshlib_positive_variable_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    assert [(origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) for origin in result["origins"][0]] == [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 3),
        (0, 0),
    ]
    assert all(not origin["is_intersection"] for origin in result["origins"][0])
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
    ) == result


def test_offset_contours_with_origins_matches_meshlib_mixed_signed_variable_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, -0.10, 0.30, -0.20, 0.20]],
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 15
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected = [
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.958763, 0.043814, True),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.084211, 0.921053, True),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, False),
    ]
    for origin, expected_origin in zip(result["origins"][0], expected):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == expected_origin[0]
        assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == expected_origin[1]
        assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == expected_origin[2]
        assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == expected_origin[3]
        assert abs(origin["l_ratio"] - expected_origin[4]) <= 1e-6
        assert abs(origin["u_ratio"] - expected_origin[5]) <= 1e-6
        assert origin["is_intersection"] is expected_origin[6]
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, -0.10, 0.30, -0.20, 0.20]],
    ) == result


def test_offset_contours_with_origins_matches_meshlib_negative_variable_intersection_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected = [
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.810474, 0.790524),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.214106, 0.760705),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
    ]
    for origin, expected_origin in zip(result["origins"][0], expected):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == expected_origin[0]
        assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == expected_origin[1]
        assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == expected_origin[2]
        assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == expected_origin[3]
        assert origin["l_ratio"] == pytest.approx(expected_origin[4], abs=1e-6)
        assert origin["u_ratio"] == pytest.approx(expected_origin[5], abs=1e-6)
        assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    ) == result


def test_offset_contours_with_origins_matches_meshlib_fixed_shell_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        mode="shell",
    )

    assert len(result["contours"]) == 2
    assert len(result["origins"]) == 2
    assert len(result["origins"][0]) == len(result["contours"][0])
    assert len(result["origins"][1]) == len(result["contours"][1])
    expected_inner = [
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.875, 0.125),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.125, 0.125),
        ((0, 0), (0, 3), (0, 2), (0, 3), 0.875, 0.875),
        ((0, 1), (0, 2), (0, 2), (0, 3), 0.875, 0.125),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.875, 0.125),
    ]
    for origin, expected_origin in zip(result["origins"][1], expected_inner):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == expected_origin[0]
        assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == expected_origin[1]
        assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == expected_origin[2]
        assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == expected_origin[3]
        assert origin["l_ratio"] == pytest.approx(expected_origin[4])
        assert origin["u_ratio"] == pytest.approx(expected_origin[5])
        assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        mode="shell",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_variable_shell_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    )

    assert len(result["contours"]) == 2
    assert len(result["origins"]) == 2
    assert len(result["origins"][0]) == len(result["contours"][0])
    assert len(result["origins"][1]) == len(result["contours"][1])
    expected_inner = [
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.214106, 0.760705),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.810474, 0.790524),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
    ]
    for origin, expected_origin in zip(result["origins"][1], expected_inner):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == expected_origin[0]
        assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == expected_origin[1]
        assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == expected_origin[2]
        assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == expected_origin[3]
        assert origin["l_ratio"] == pytest.approx(expected_origin[4], abs=1e-6)
        assert origin["u_ratio"] == pytest.approx(expected_origin[5], abs=1e-6)
        assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_round_corner_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [-0.0, -0.200000, 0.0],
            [-0.077044, -0.185038, 0.0],
            [-0.132326, -0.163134, 0.0],
            [-0.169086, -0.128711, 0.0],
            [-0.190564, -0.076192, 0.0],
            [-0.200000, -0.0, 0.0],
            [-0.300000, 2.000000, 0.0],
            [-0.294688, 2.116414, 0.0],
            [-0.263973, 2.199443, 0.0],
            [-0.205915, 2.254265, 0.0],
            [-0.118571, 2.286058, 0.0],
            [-0.0, 2.300000, 0.0],
            [2.000000, 2.400000, 0.0],
            [2.155218, 2.392917, 0.0],
            [2.265924, 2.351964, 0.0],
            [2.339020, 2.274553, 0.0],
            [2.381411, 2.158094, 0.0],
            [2.400000, 2.000000, 0.0],
            [2.500000, -0.0, 0.0],
            [2.490793, -0.201161, 0.0],
            [2.438895, -0.353819, 0.0],
            [2.341601, -0.455896, 0.0],
            [2.196205, -0.505316, 0.0],
            [2.000000, -0.500000, 0.0],
            [-0.0, -0.200000, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_sharp_corner_max_angle_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.000000, -0.200000, 0.0],
            [-0.063326, -0.190501, 0.0],
            [-0.197180, -0.056394, 0.0],
            [-0.200000, -0.000000, 0.0],
            [-0.300000, 2.000000, 0.0],
            [-0.303800, 2.076005, 0.0],
            [-0.084555, 2.295772, 0.0],
            [0.000000, 2.300000, 0.0],
            [2.000000, 2.400000, 0.0],
            [2.101339, 2.405067, 0.0],
            [2.394363, 2.112740, 0.0],
            [2.400000, 2.000000, 0.0],
            [2.500000, -0.000000, 0.0],
            [2.506352, -0.127042, 0.0],
            [2.115096, -0.517264, 0.0],
            [2.000000, -0.500000, 0.0],
            [0.000000, -0.200000, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    ) == result


def test_offset_contours_matches_meshlib_closed_negative_offset_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.25, 0.25, 0.0],
            [0.25, 1.75, 0.0],
            [1.75, 1.75, 0.0],
            [1.75, 0.25, 0.0],
            [0.25, 0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_negative_offset_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.211587, 0.231738, 0.0],
            [0.284289, 1.685786, 0.0],
            [1.581047, 1.620948, 0.0],
            [1.521411, 0.428212, 0.0],
            [0.211587, 0.231738, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    ) == result


def test_offset_contours_matches_meshlib_closed_sharp_corner_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        corner_type="sharp",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, -0.25, 0.0],
            [-0.25, -0.25, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.25, 0.0],
            [0.0, 2.25, 0.0],
            [2.0, 2.25, 0.0],
            [2.25, 2.25, 0.0],
            [2.25, 2.0, 0.0],
            [2.25, 0.0, 0.0],
            [2.25, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        corner_type="sharp",
    ) == result


def test_offset_contours_matches_meshlib_closed_sharp_corner_max_angle_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, -0.25, 0.0],
            [-0.066987, -0.25, 0.0],
            [-0.25, -0.066987, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.066988, 0.0],
            [-0.066987, 2.25, 0.0],
            [0.0, 2.25, 0.0],
            [2.0, 2.25, 0.0],
            [2.066988, 2.25, 0.0],
            [2.25, 2.066988, 0.0],
            [2.25, 2.0, 0.0],
            [2.25, 0.0, 0.0],
            [2.25, -0.066987, 0.0],
            [2.066988, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    ) == result


def test_offset_contours_matches_meshlib_default_3d_z_restore_relaxation_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offset=0.25,
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.000000, -0.250000, 0.111672],
            [-0.077254, -0.237764, 0.000000],
            [-0.146946, -0.202254, 0.000000],
            [-0.202254, -0.146946, 0.000000],
            [-0.237764, -0.077254, 0.000000],
            [-0.250000, 0.000000, 0.037224],
            [-0.250000, 2.000000, 1.962776],
            [-0.237764, 2.077254, 2.000000],
            [-0.202254, 2.146946, 2.000000],
            [-0.146946, 2.202254, 2.000000],
            [-0.077254, 2.237764, 2.000000],
            [0.000000, 2.250000, 2.037224],
            [2.000000, 2.250000, 3.962776],
            [2.077254, 2.237764, 4.000000],
            [2.146946, 2.202254, 4.000000],
            [2.202254, 2.146946, 4.000000],
            [2.237764, 2.077254, 4.000000],
            [2.250000, 2.000000, 4.037224],
            [2.250000, 0.000000, 5.962776],
            [2.237764, -0.077254, 6.000000],
            [2.202254, -0.146946, 6.000000],
            [2.146946, -0.202254, 6.000000],
            [2.077254, -0.237764, 6.000000],
            [2.000000, -0.250000, 5.888328],
            [0.000000, -0.250000, 0.111672],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offset=0.25,
    ) == result


def test_offset_contours_exposes_meshlib_restore_z_relax_iterations() -> None:
    contour = [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]]

    result = offset_contours(contour, offset=0.25, relax_iterations=0)

    assert len(result) == 1
    np.testing.assert_allclose(
        [point[2] for point in result[0]],
        [
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0,
            2.0,
            2.0,
            2.0,
            2.0,
            2.0,
            4.0,
            4.0,
            4.0,
            4.0,
            4.0,
            4.0,
            6.0,
            6.0,
            6.0,
            6.0,
            6.0,
            6.0,
            0.0,
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(contour, offset=0.25, relax_iterations=0) == result


def test_offset_contours_exposes_meshlib_constant_z_callback_mode() -> None:
    contour = [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]]

    result = offset_contours(contour, offset=0.25, z_restore="constant", z_value=9.0)

    assert len(result) == 1
    assert all(point[2] == pytest.approx(9.0) for point in result[0])
    assert (
        GeometrySDK().offset_contours(
            contour,
            offset=0.25,
            z_restore="constant",
            z_value=9.0,
        )
        == result
    )

    with pytest.raises(ValueError, match="z_value"):
        offset_contours(contour, offset=0.25, z_restore="constant")


def test_offset_contours_exposes_meshlib_custom_z_callback_mode() -> None:
    contour = [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]]
    z_values = [[10.0, 12.0, 14.0, 16.0, 10.0]]

    result = offset_contours(
        contour,
        offset=0.25,
        z_restore="custom",
        z_values=z_values,
        relax_iterations=0,
    )

    assert len(result) == 1
    assert np.allclose(
        [point[2] for point in result[0]],
        [
            10.0,
            10.0,
            10.0,
            10.0,
            10.0,
            10.0,
            12.0,
            12.0,
            12.0,
            12.0,
            12.0,
            12.0,
            14.0,
            14.0,
            14.0,
            14.0,
            14.0,
            14.0,
            16.0,
            16.0,
            16.0,
            16.0,
            16.0,
            16.0,
            10.0,
        ],
        atol=1e-8,
    )
    assert (
        GeometrySDK().offset_contours(
            contour,
            offset=0.25,
            z_restore="callable",
            z_values=z_values,
            relax_iterations=0,
        )
        == result
    )

    with pytest.raises(ValueError, match="z_values"):
        offset_contours(contour, offset=0.25, z_restore="custom")


def test_offset_contours_exposes_meshlib_callable_z_callback_context() -> None:
    contour = [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]]
    calls: list[tuple[tuple[int, int], int]] = []

    def z_callback(point, offset_index, origin) -> float:
        calls.append(
            (
                (offset_index["contour_id"], offset_index["vert_id"]),
                origin["l_org"]["vert_id"],
            )
        )
        return float(point[0] + 10.0 * point[1] + offset_index["vert_id"] * 0.01 + origin["l_org"]["vert_id"])

    result = offset_contours(
        contour,
        offset=0.25,
        z_restore="zCallback",
        z_values=z_callback,
        relax_iterations=0,
    )

    assert len(result) == 1
    assert len(calls) == len(result[0])
    assert calls[0] == ((0, 0), 0)
    assert calls[6] == ((0, 6), 1)
    assert calls[12] == ((0, 12), 2)
    np.testing.assert_allclose(
        [point[2] for point in result[0]],
        [
            point[0] + 10.0 * point[1] + index * 0.01 + calls[index][1]
            for index, point in enumerate(result[0])
        ],
        atol=1e-8,
    )
    assert GeometrySDK().offset_contours(
        contour,
        offset=0.25,
        z_restore="callable",
        z_values=z_callback,
        relax_iterations=0,
    ) == result


def test_offset_contours_matches_meshlib_variable_shell_3d_z_restore_relaxation_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    )

    assert len(result) == 2
    np.testing.assert_allclose(
        result[1],
        [
            [0.284289, 1.685786, 2.196915],
            [0.211587, 0.231738, 2.070361],
            [1.521411, 0.428212, 3.713774],
            [1.581047, 1.620948, 3.817585],
            [0.284289, 1.685786, 2.196915],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_variable_negative_offset_3d_z_restore_relaxation_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.211587, 0.231738, 2.070361],
            [0.284289, 1.685786, 2.196915],
            [1.581047, 1.620948, 3.817585],
            [1.521411, 0.428212, 3.713774],
            [0.211587, 0.231738, 2.070361],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
    ) == result


def test_offset_contours_matches_meshlib_variable_sharp_max_angle_3d_z_restore_relaxation_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.000000, -0.200000, 0.092073],
            [-0.063326, -0.190501, 0.000000],
            [-0.197180, -0.056394, 0.000000],
            [-0.200000, -0.000000, 0.027423],
            [-0.300000, 2.000000, 1.963389],
            [-0.303800, 2.076005, 2.000000],
            [-0.084555, 2.295772, 2.000000],
            [0.000000, 2.300000, 2.040563],
            [2.000000, 2.400000, 3.951774],
            [2.101339, 2.405067, 4.000000],
            [2.394363, 2.112740, 4.000000],
            [2.400000, 2.000000, 4.053362],
            [2.500000, -0.000000, 5.940273],
            [2.506352, -0.127042, 6.000000],
            [2.115096, -0.517264, 6.000000],
            [2.000000, -0.500000, 5.836751],
            [0.000000, -0.200000, 0.092073],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0, 0.0), (0.0, 2.0, 2.0), (2.0, 2.0, 4.0), (2.0, 0.0, 6.0), (0.0, 0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, -0.10, 0.30, -0.20, 0.20]],
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.000000, -0.200000, 0.0],
            [-0.079418, -0.204737, 0.0],
            [-0.140350, -0.185030, 0.0],
            [-0.181574, -0.142955, 0.0],
            [-0.201865, -0.080587, 0.0],
            [-0.200000, 0.000000, 0.0],
            [0.087629, 1.917526, 0.0],
            [2.000000, 2.300000, 0.0],
            [2.121161, 2.306700, 0.0],
            [2.216629, 2.276328, 0.0],
            [2.281516, 2.212606, 0.0],
            [2.310935, 2.119256, 0.0],
            [2.300000, 2.000000, 0.0],
            [1.842105, 0.168421, 0.0],
            [0.000000, -0.200000, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, -0.10, 0.30, -0.20, 0.20]],
    ) == result


def test_offset_contours_matches_meshlib_closed_shell_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        mode="shell",
    )

    assert len(result) == 2
    np.testing.assert_allclose(
        result[1],
        [
            [0.25, 1.75, 0.0],
            [0.25, 0.25, 0.0],
            [1.75, 0.25, 0.0],
            [1.75, 1.75, 0.0],
            [0.25, 1.75, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=0.25,
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_shell_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    )

    assert len(result) == 2
    np.testing.assert_allclose(
        result[1],
        [
            [0.284289, 1.685786, 0.0],
            [0.211587, 0.231738, 0.0],
            [1.521411, 0.428212, 0.0],
            [1.581047, 1.620948, 0.0],
            [0.284289, 1.685786, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_sharp_shell_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    )

    assert len(result) == 2
    np.testing.assert_allclose(
        result[0],
        [
            [0.000000, -0.200000, 0.0],
            [-0.063326, -0.190501, 0.0],
            [-0.197180, -0.056394, 0.0],
            [-0.200000, -0.000000, 0.0],
            [-0.300000, 2.000000, 0.0],
            [-0.303800, 2.076005, 0.0],
            [-0.084555, 2.295772, 0.0],
            [0.000000, 2.300000, 0.0],
            [2.000000, 2.400000, 0.0],
            [2.101339, 2.405067, 0.0],
            [2.394363, 2.112740, 0.0],
            [2.400000, 2.000000, 0.0],
            [2.500000, -0.000000, 0.0],
            [2.506352, -0.127042, 0.0],
            [2.115096, -0.517264, 0.0],
            [2.000000, -0.500000, 0.0],
            [0.000000, -0.200000, 0.0],
        ],
        atol=1e-5,
    )
    np.testing.assert_allclose(
        result[1],
        [
            [0.284289, 1.685786, 0.0],
            [0.211587, 0.231738, 0.0],
            [1.521411, 0.428212, 0.0],
            [1.581047, 1.620948, 0.0],
            [0.284289, 1.685786, 0.0],
        ],
        atol=1e-5,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[0.20, 0.30, 0.40, 0.50, 0.20]],
        mode="shell",
        corner_type="sharp",
        max_sharp_angle=float(np.pi / 6.0),
    ) == result


def test_offset_contours_matches_meshlib_closed_negative_shell_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
        mode="shell",
    )

    assert result == []
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offset=-0.25,
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_closed_variable_negative_shell_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
        mode="shell",
    )

    assert result == []
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0), (0.0, 0.0)]],
        offsets=[[-0.20, -0.30, -0.40, -0.50, -0.20]],
        mode="shell",
    ) == result


def test_offset_contours_matches_meshlib_open_round_end_contract() -> None:
    result = offset_contours([[(0.0, 0.0), (2.0, 0.0)]], offset=0.25)

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.077254, 0.237764, 0.0],
            [2.146946, 0.202254, 0.0],
            [2.202254, 0.146946, 0.0],
            [2.237764, 0.077254, 0.0],
            [2.25, 0.0, 0.0],
            [2.237764, -0.077254, 0.0],
            [2.202254, -0.146946, 0.0],
            [2.146946, -0.202254, 0.0],
            [2.077254, -0.237764, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [-0.077254, -0.237764, 0.0],
            [-0.146946, -0.202254, 0.0],
            [-0.202254, -0.146946, 0.0],
            [-0.237764, -0.077254, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.237764, 0.077254, 0.0],
            [-0.202254, 0.146946, 0.0],
            [-0.146946, 0.202254, 0.0],
            [-0.077254, 0.237764, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )


def test_offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract() -> None:
    result = offset_contours_with_origins([[(0.0, 0.0), (2.0, 0.0)]], offset=0.25)

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    assert [(origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) for origin in result["origins"][0]] == [
        (0, 0),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ]
    assert all(not origin["is_intersection"] for origin in result["origins"][0])
    assert GeometrySDK().offset_contours_with_origins([[(0.0, 0.0), (2.0, 0.0)]], offset=0.25) == result


def test_offset_contours_with_origins_matches_meshlib_open_fixed_round_end_bend_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="round",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 30
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.75, 1.0, 0.0],
            [0.762236, 1.077254, 0.0],
            [0.797746, 1.146946, 0.0],
            [0.853054, 1.202254, 0.0],
            [0.922746, 1.237764, 0.0],
            [1.0, 1.25, 0.0],
            [1.077254, 1.237764, 0.0],
            [1.146946, 1.202254, 0.0],
            [1.202254, 1.146946, 0.0],
            [1.237764, 1.077254, 0.0],
            [1.25, 1.0, 0.0],
            [1.25, 0.0, 0.0],
            [1.237764, -0.077254, 0.0],
            [1.202254, -0.146946, 0.0],
            [1.146946, -0.202254, 0.0],
            [1.077254, -0.237764, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [-0.077254, -0.237764, 0.0],
            [-0.146946, -0.202254, 0.0],
            [-0.202254, -0.146946, 0.0],
            [-0.237764, -0.077254, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.237764, 0.077254, 0.0],
            [-0.202254, 0.146946, 0.0],
            [-0.146946, 0.202254, 0.0],
            [-0.077254, 0.237764, 0.0],
            [0.0, 0.25, 0.0],
            [0.75, 0.25, 0.0],
            [0.75, 1.0, 0.0],
        ],
        atol=1e-6,
    )
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected_lorg_vertices = [2] * 11 + [1] * 6 + [0] * 12 + [2]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 28:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 1)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.75, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.75, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="round",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_fixed_round_end_zig_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (0.2, 0.4), (1.2, 0.8)]],
        offset=0.25,
        end_type="round",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 40
    np.testing.assert_allclose(result["contours"][0][38], [0.003986, 0.25, 0.0], atol=1e-6)
    origin = result["origins"][0][38]
    assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, 0)
    assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 1)
    assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
    assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 2)
    assert origin["l_ratio"] == pytest.approx(0.003986, abs=1e-6)
    assert origin["u_ratio"] == pytest.approx(0.615854, abs=1e-6)
    assert origin["is_intersection"] is True
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (0.2, 0.4), (1.2, 0.8)]],
        offset=0.25,
        end_type="round",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_contract() -> None:
    result = offset_contours([[(0.0, 0.0), (2.0, 0.0)]], offset=0.25, end_type="cut")

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours([[(0.0, 0.0), (2.0, 0.0)]], offset=0.25, end_type="cut") == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, -1.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [1.25, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, -0.25, 0.0],
            [1.25, -0.25, 0.0],
            [1.25, -1.0, 0.0],
            [0.75, -1.0, 0.0],
            [0.75, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [0.75, 0.25, 0.0],
            [0.75, 1.0, 0.0],
            [1.25, 1.0, 0.0],
            [1.25, 0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((1, 0), (1, 1), (0, 0), (0, 1), 0.625, 0.625),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 0), (0, 1), 0.375, 0.625),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 1), (1, 0), 0.375, 0.625),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 1), (1, 0), 0.375, 0.375),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 0), (0, 1), 0.625, 0.625),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, -1.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.1), (3.0, 0.1)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [1.0, 0.25, 0.0],
            [1.0, 0.35, 0.0],
            [3.0, 0.35, 0.0],
            [3.0, -0.15, 0.0],
            [2.0, -0.15, 0.0],
            [2.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.1), (3.0, 0.1)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.1), (3.0, 0.1)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [1.0, 0.25, 0.0],
            [1.0, 0.35, 0.0],
            [3.0, 0.35, 0.0],
            [3.0, -0.15, 0.0],
            [2.0, -0.15, 0.0],
            [2.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 0), (0, 1), 0.8, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 1), (0, 1), (1, 0), (1, 1), 0.2, 0.5),
        ((0, 1), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.1), (3.0, 0.1)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(2.0, 0.0), (4.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [4.0, 0.25, 0.0],
            [4.0, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(2.0, 0.0), (4.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


@pytest.mark.parametrize(
    ("contours", "expected_points", "expected_origins"),
    [
        (
            [[(0.0, 0.0), (2.0, 0.0)], [(2.0, 0.0), (4.0, 0.0)]],
            [
                [0.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [4.0, 0.25, 0.0],
                [4.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [0.0, -0.25, 0.0],
                [0.0, 0.25, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 1), (1, 0), (1, 1), 1.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 0), (0, 0), (0, 1), 0.0, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(0.0, 0.0), (2.0, 0.0)], [(4.0, 0.0), (2.0, 0.0)]],
            [
                [0.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [4.0, 0.25, 0.0],
                [4.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [0.0, -0.25, 0.0],
                [0.0, 0.25, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 1), (1, 1), (1, 0), 1.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 1), (1, 1), 1.0, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(2.0, 0.0), (0.0, 0.0)], [(2.0, 0.0), (4.0, 0.0)]],
            [
                [0.0, -0.25, 0.0],
                [0.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [4.0, 0.25, 0.0],
                [4.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [0.0, -0.25, 0.0],
            ],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 1), (0, 0), (0, 0), 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 0), (0, 1), (0, 0), 0.0, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(2.0, 0.0), (0.0, 0.0)], [(4.0, 0.0), (2.0, 0.0)]],
            [
                [0.0, -0.25, 0.0],
                [0.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [2.0, 0.25, 0.0],
                [4.0, 0.25, 0.0],
                [4.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [2.0, -0.25, 0.0],
                [0.0, -0.25, 0.0],
            ],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 0), (0, 0), (0, 0), 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 1), (1, 1), 1.0, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
    ],
)
def test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract(
    contours: list[list[tuple[float, float]]],
    expected_points: list[list[float]],
    expected_origins: list[tuple[tuple[int, int], tuple[int, int] | None, tuple[int, int] | None, tuple[int, int] | None, float, float]],
) -> None:
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(result["contours"][0], expected_points, atol=1e-6)

    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        contours,
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 1), (0, 0), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 1), (0, 1), 0.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 4.0), (0.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 1), (1, 1), (0, 1), (0, 0), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), (1, 0), (0, 1), (0, 1), 0.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 4.0), (0.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 0), (0, 1), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 0), (0, 0), 0.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 2.0), (0.0, 4.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (1.0, 1.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [1.823223, 2.176777, 0.0],
            [2.176777, 1.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (1.0, 1.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 1.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [1.823223, 2.176777, 0.0],
            [2.176777, 1.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 1), (0, 1), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 1.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 1.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 1.0)], [(2.0, 2.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [1.823223, 2.176777, 0.0],
            [2.176777, 1.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 1), (1, 0), (0, 1), (0, 1), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 1), (1, 1), 1.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 1.0)], [(2.0, 2.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(1.0, 1.0), (0.0, 0.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [1.823223, 2.176777, 0.0],
            [2.176777, 1.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 0), (0, 0), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 1), (0, 0), (1, 0), (1, 0), 1.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(1.0, 1.0), (0.0, 0.0)], [(1.0, 1.0), (2.0, 2.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract() -> None:
    shift = np.sqrt(0.5) * 0.1
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0 - shift, 1.0 + shift), (3.0 - shift, 3.0 + shift)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [2.106066, 1.893934, 0.0],
            [2.176777, 1.823223, 0.0],
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.752513, 1.247487, 0.0],
            [2.752513, 3.247487, 0.0],
            [3.106066, 2.893934, 0.0],
            [2.106066, 1.893934, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0 - shift, 1.0 + shift), (3.0 - shift, 3.0 + shift)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract() -> None:
    shift = np.sqrt(0.5) * 0.1
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0 - shift, 1.0 + shift), (3.0 - shift, 3.0 + shift)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [2.106066, 1.893934, 0.0],
            [2.176777, 1.823223, 0.0],
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.752513, 1.247487, 0.0],
            [2.752513, 3.247487, 0.0],
            [3.106066, 2.893934, 0.0],
            [2.106066, 1.893934, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((1, 0), (1, 1), (0, 1), (0, 1), 0.5, 0.8),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 0.2),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 1), (0, 1), 0.5, 0.8),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0 - shift, 1.0 + shift), (3.0 - shift, 3.0 + shift)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0, 1.0), (3.0, 3.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.823223, 3.176777, 0.0],
            [3.176777, 2.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0, 1.0), (3.0, 3.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0, 1.0), (3.0, 3.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.823223, 3.176777, 0.0],
            [3.176777, 2.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 2.0)], [(1.0, 1.0), (3.0, 3.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract() -> None:
    contours = [
        [(0.0, 0.0), (2.0, 2.0)],
        [(1.0, 1.0), (3.0, 3.0)],
        [(2.0, 2.0), (4.0, 4.0)],
    ]
    result = offset_contours(contours, offset=0.25, end_type="cut")

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.023223, 2.376777, 0.0],
            [3.823223, 4.176777, 0.0],
            [4.176777, 3.823223, 0.0],
            [2.376777, 2.023223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(contours, offset=0.25, end_type="cut") == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    contours = [
        [(0.0, 0.0), (2.0, 2.0)],
        [(1.0, 1.0), (3.0, 3.0)],
        [(2.0, 2.0), (4.0, 4.0)],
    ]
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.023223, 2.376777, 0.0],
            [3.823223, 4.176777, 0.0],
            [4.176777, 3.823223, 0.0],
            [2.376777, 2.023223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((2, 0), (2, 1), (1, 0), (1, 1), 0.1, 0.6),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (2, 0), (2, 1), 0.6, 0.1),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(contours, offset=0.25, end_type="cut") == result


@pytest.mark.parametrize(
    ("contours", "expected_origins"),
    [
        (
            [
                [(0.0, 0.0), (2.0, 2.0)],
                [(3.0, 3.0), (1.0, 1.0)],
                [(2.0, 2.0), (4.0, 4.0)],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 1), (1, 1), 0.5, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((2, 0), (2, 1), (1, 1), (1, 0), 0.1, 0.6),
                ((2, 1), None, None, None, 0.0, 0.0),
                ((2, 1), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 0), (2, 0), (2, 1), 0.6, 0.1),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 1), (1, 1), 0.5, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [
                [(2.0, 2.0), (0.0, 0.0)],
                [(1.0, 1.0), (3.0, 3.0)],
                [(2.0, 2.0), (4.0, 4.0)],
            ],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 0), (1, 0), 0.5, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((2, 0), (2, 1), (1, 0), (1, 1), 0.1, 0.6),
                ((2, 1), None, None, None, 0.0, 0.0),
                ((2, 1), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 1), (2, 0), (2, 1), 0.6, 0.1),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 0), (1, 0), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [
                [(0.0, 0.0), (2.0, 2.0)],
                [(1.0, 1.0), (3.0, 3.0)],
                [(4.0, 4.0), (2.0, 2.0)],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((2, 1), (2, 0), (1, 0), (1, 1), 0.1, 0.6),
                ((2, 0), None, None, None, 0.0, 0.0),
                ((2, 0), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 1), (2, 1), (2, 0), 0.6, 0.1),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 0), (1, 0), 0.5, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [
                [(2.0, 2.0), (0.0, 0.0)],
                [(3.0, 3.0), (1.0, 1.0)],
                [(4.0, 4.0), (2.0, 2.0)],
            ],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 1), (1, 1), 0.5, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((2, 1), (2, 0), (1, 1), (1, 0), 0.1, 0.6),
                ((2, 0), None, None, None, 0.0, 0.0),
                ((2, 0), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 0), (2, 1), (2, 0), 0.6, 0.1),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 1), (1, 1), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
    ],
)
def test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract(
    contours: list[list[tuple[float, float]]],
    expected_origins: list[tuple[tuple[int, int], tuple[int, int] | None, tuple[int, int] | None, tuple[int, int] | None, float, float]],
) -> None:
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.023223, 2.376777, 0.0],
            [3.823223, 4.176777, 0.0],
            [4.176777, 3.823223, 0.0],
            [2.376777, 2.023223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )

    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        contours,
        offset=0.25,
        end_type="cut",
    ) == result


@pytest.mark.parametrize(
    ("contours", "expected_origins"),
    [
        (
            [[(0.0, 0.0), (2.0, 2.0)], [(3.0, 3.0), (1.0, 1.0)]],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 1), (1, 1), 0.5, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 0), (0, 1), (1, 1), (1, 1), 0.5, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(2.0, 2.0), (0.0, 0.0)], [(1.0, 1.0), (3.0, 3.0)]],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 0), (1, 0), 0.5, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 0), (1, 0), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(2.0, 2.0), (0.0, 0.0)], [(3.0, 3.0), (1.0, 1.0)]],
            [
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 1), (1, 1), 0.5, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((0, 1), (0, 0), (1, 1), (1, 1), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
            ],
        ),
    ],
)
def test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract(
    contours: list[list[tuple[float, float]]],
    expected_origins: list[tuple[tuple[int, int], tuple[int, int] | None, tuple[int, int] | None, tuple[int, int] | None, float, float]],
) -> None:
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.823223, 3.176777, 0.0],
            [3.176777, 2.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ],
        atol=1e-6,
    )

    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        contours,
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), (0, 1), (1, 0), (1, 1), 1.0, 0.5),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 0), (0, 1), 0.0, 0.5),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    contours = [
        [(0.0, 0.0), (2.0, 0.0)],
        [(1.0, 0.0), (3.0, 0.0)],
        [(2.0, 0.0), (4.0, 0.0)],
    ]
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [4.0, 0.25, 0.0],
            [4.0, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [2.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), (0, 1), (1, 0), (1, 1), 1.0, 0.5),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), (1, 1), (2, 0), (2, 1), 1.0, 0.5),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 0), None, None, None, 0.0, 0.0),
        ((2, 0), (2, 0), (1, 0), (1, 1), 0.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 0), (0, 1), 0.0, 0.5),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(contours, offset=0.25, end_type="cut") == result


def test_offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract() -> None:
    contours = [
        [(0.0, 0.0), (0.0, 2.0)],
        [(0.0, 1.0), (0.0, 3.0)],
        [(0.0, 2.0), (0.0, 4.0)],
    ]
    result = offset_contours(contours, offset=0.25, end_type="cut")

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [-0.25, 0.0, 0.0],
            [-0.25, 1.0, 0.0],
            [-0.25, 1.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 3.0, 0.0],
            [0.25, 3.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(contours, offset=0.25, end_type="cut") == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    contours = [
        [(0.0, 0.0), (0.0, 2.0)],
        [(0.0, 1.0), (0.0, 3.0)],
        [(0.0, 2.0), (0.0, 4.0)],
    ]
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.25, 0.0, 0.0],
            [-0.25, 1.0, 0.0],
            [-0.25, 1.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 2.0, 0.0],
            [-0.25, 4.0, 0.0],
            [0.25, 4.0, 0.0],
            [0.25, 3.0, 0.0],
            [0.25, 3.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 2.0, 0.0],
            [0.25, 0.0, 0.0],
            [-0.25, 0.0, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 1), (0, 0), 0.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((2, 0), (2, 0), (1, 1), (1, 0), 0.0, 0.5),
        ((2, 0), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 0), (2, 1), (1, 1), (1, 1), 0.5, 1.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 1), (0, 1), 0.5, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(contours, offset=0.25, end_type="cut") == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(3.0, 0.0), (1.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), (0, 1), (1, 1), (1, 0), 1.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 0), (0, 1), (1, 1), (1, 1), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (2.0, 0.0)], [(3.0, 0.0), (1.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(2.0, 0.0), (0.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )

    expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 1), (0, 0), (0, 0), 0.5, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), (1, 0), (0, 1), (0, 0), 0.0, 0.5),
        ((0, 1), None, None, None, 0.0, 0.0),
    ]
    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        [[(2.0, 0.0), (0.0, 0.0)], [(1.0, 0.0), (3.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract() -> None:
    result = offset_contours(
        [[(2.0, 0.0), (0.0, 0.0)], [(3.0, 0.0), (1.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [2.0, 0.25, 0.0],
            [3.0, 0.25, 0.0],
            [3.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(2.0, 0.0), (0.0, 0.0)], [(3.0, 0.0), (1.0, 0.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


@pytest.mark.parametrize(
    ("contours", "expected_points"),
    [
        (
            [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 1.0), (0.0, 3.0)]],
            [
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
            ],
        ),
        (
            [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 3.0), (0.0, 1.0)]],
            [
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
            ],
        ),
        (
            [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 1.0), (0.0, 3.0)]],
            [
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
            ],
        ),
        (
            [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 3.0), (0.0, 1.0)]],
            [
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
            ],
        ),
    ],
)
def test_offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract(
    contours: list[list[tuple[float, float]]],
    expected_points: list[list[float]],
) -> None:
    result = offset_contours(contours, offset=0.25, end_type="cut")

    assert len(result) == 1
    np.testing.assert_allclose(result[0], expected_points, atol=1e-6)
    assert GeometrySDK().offset_contours(contours, offset=0.25, end_type="cut") == result


@pytest.mark.parametrize(
    ("contours", "expected_points", "expected_origins"),
    [
        (
            [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 1.0), (0.0, 3.0)]],
            [
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 0), (0, 1), (0, 0), 0.0, 0.5),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 1), (0, 1), (0, 1), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(0.0, 0.0), (0.0, 2.0)], [(0.0, 3.0), (0.0, 1.0)]],
            [
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 1), (0, 1), (0, 0), 0.0, 0.5),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 0), (0, 1), (0, 1), 0.5, 1.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 1.0), (0.0, 3.0)]],
            [
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 0), (0, 0), (0, 1), 0.0, 0.5),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), (1, 1), (0, 0), (0, 0), 0.5, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
        (
            [[(0.0, 2.0), (0.0, 0.0)], [(0.0, 3.0), (0.0, 1.0)]],
            [
                [0.25, 2.0, 0.0],
                [0.25, 0.0, 0.0],
                [-0.25, 0.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 1.0, 0.0],
                [-0.25, 3.0, 0.0],
                [0.25, 3.0, 0.0],
                [0.25, 2.0, 0.0],
                [0.25, 2.0, 0.0],
            ],
            [
                ((0, 0), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((0, 1), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 1), (0, 0), (0, 1), 0.0, 0.5),
                ((1, 1), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 0), None, None, None, 0.0, 0.0),
                ((1, 1), (1, 0), (0, 0), (0, 0), 0.5, 1.0),
                ((0, 0), None, None, None, 0.0, 0.0),
            ],
        ),
    ],
)
def test_offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract(
    contours: list[list[tuple[float, float]]],
    expected_points: list[list[float]],
    expected_origins: list[tuple[tuple[int, int], tuple[int, int] | None, tuple[int, int] | None, tuple[int, int] | None, float, float]],
) -> None:
    result = offset_contours_with_origins(contours, offset=0.25, end_type="cut")

    assert len(result["contours"]) == 1
    assert len(result["origins"]) == 1
    np.testing.assert_allclose(result["contours"][0], expected_points, atol=1e-6)

    assert len(result["origins"][0]) == len(expected_origins)
    for origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio) in zip(result["origins"][0], expected_origins):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == l_org
        if l_dest is None:
            assert not origin["is_intersection"]
        else:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == l_dest
            assert origin["is_intersection"]
        if u_org is not None:
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == u_org
        if u_dest is not None:
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == u_dest
        assert abs(origin["l_ratio"] - l_ratio) <= 1e-6
        assert abs(origin["u_ratio"] - u_ratio) <= 1e-6

    assert GeometrySDK().offset_contours_with_origins(
        contours,
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_matches_meshlib_open_variable_cut_end_contract() -> None:
    result = offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)]],
        offsets=[[0.25, 0.5]],
        end_type="cut",
    )

    assert len(result) == 1
    np.testing.assert_allclose(
        result[0],
        [
            [0.0, 0.25, 0.0],
            [2.0, 0.5, 0.0],
            [2.0, -0.5, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ],
        atol=1e-6,
    )
    assert GeometrySDK().offset_contours(
        [[(0.0, 0.0), (2.0, 0.0)]],
        offsets=[[0.25, 0.5]],
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_fixed_cut_end_bend_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 12
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.75, 1.0, 0.0],
            [1.25, 1.0, 0.0],
            [1.25, 0.0, 0.0],
            [1.237764, -0.077254, 0.0],
            [1.202254, -0.146946, 0.0],
            [1.146946, -0.202254, 0.0],
            [1.077254, -0.237764, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.25, 0.0],
            [0.0, 0.25, 0.0],
            [0.75, 0.25, 0.0],
            [0.75, 1.0, 0.0],
        ],
        atol=1e-6,
    )
    expected_lorg_vertices = [2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 2]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 10:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 1)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.75, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.75, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offset=0.25,
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_variable_cut_end_bend_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.18, 0.25, 0.32]],
        end_type="cut",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 12
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.68, 1.0, 0.0],
            [1.32, 1.0, 0.0],
            [1.25, 0.0, 0.0],
            [1.236928, -0.099081, 0.0],
            [1.210212, -0.172573, 0.0],
            [1.165032, -0.221524, 0.0],
            [1.096567, -0.246984, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.18, 0.0],
            [0.0, 0.18, 0.0],
            [0.733804, 0.231366, 0.0],
            [0.68, 1.0, 0.0],
        ],
        atol=1e-6,
    )
    expected_lorg_vertices = [2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 2]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 10:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 1)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.733804, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.768634, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.18, 0.25, 0.32]],
        end_type="cut",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_variable_round_end_self_overlap_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.25, 0.40, 0.20]],
        end_type="round",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 30
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.670103, 0.350515, 0.0],
            [0.8, 1.0, 0.0],
            [0.823908, 1.079427, 0.0],
            [0.858545, 1.141204, 0.0],
            [0.901226, 1.18533, 0.0],
            [0.949272, 1.211805, 0.0],
            [1.0, 1.220631, 0.0],
            [1.050728, 1.211805, 0.0],
            [1.098774, 1.18533, 0.0],
            [1.141456, 1.141204, 0.0],
            [1.176092, 1.079427, 0.0],
            [1.2, 1.0, 0.0],
            [1.4, 0.0, 0.0],
            [1.409474, -0.158835, 0.0],
            [1.370061, -0.2807, 0.0],
            [1.285911, -0.363147, 0.0],
            [1.161174, -0.40373, 0.0],
            [1.0, -0.4, 0.0],
            [0.0, -0.25, 0.0],
            [-0.10013, -0.223984, 0.0],
            [-0.178009, -0.181979, 0.0],
            [-0.233636, -0.127982, 0.0],
            [-0.267013, -0.06599, 0.0],
            [-0.278138, 0.0, 0.0],
            [-0.267013, 0.06599, 0.0],
            [-0.233636, 0.127982, 0.0],
            [-0.178009, 0.181979, 0.0],
            [-0.10013, 0.223984, 0.0],
            [0.0, 0.25, 0.0],
            [0.670103, 0.350515, 0.0],
        ],
        atol=1e-6,
    )
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected_lorg_vertices = [1] + [2] * 11 + [1] * 6 + [0] * 11 + [1]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index in (0, len(result["origins"][0]) - 1):
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 2)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 0)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.350516, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.670103, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.25, 0.40, 0.20]],
        end_type="round",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_variable_increasing_round_end_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.18, 0.25, 0.32]],
        end_type="round",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 30
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [0.68, 1.0, 0.0],
            [0.69068, 1.129284, 0.0],
            [0.736907, 1.229838, 0.0],
            [0.809793, 1.301662, 0.0],
            [0.900453, 1.344756, 0.0],
            [1.0, 1.359121, 0.0],
            [1.099547, 1.344756, 0.0],
            [1.190207, 1.301662, 0.0],
            [1.263093, 1.229838, 0.0],
            [1.30932, 1.129284, 0.0],
            [1.32, 1.0, 0.0],
            [1.25, 0.0, 0.0],
            [1.236928, -0.099081, 0.0],
            [1.210212, -0.172573, 0.0],
            [1.165032, -0.221524, 0.0],
            [1.096567, -0.246984, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.18, 0.0],
            [-0.072722, -0.165848, 0.0],
            [-0.129284, -0.13713, 0.0],
            [-0.169685, -0.097489, 0.0],
            [-0.193925, -0.050565, 0.0],
            [-0.202006, 0.0, 0.0],
            [-0.193925, 0.050565, 0.0],
            [-0.169685, 0.097489, 0.0],
            [-0.129284, 0.13713, 0.0],
            [-0.072722, 0.165848, 0.0],
            [0.0, 0.18, 0.0],
            [0.733804, 0.231366, 0.0],
            [0.68, 1.0, 0.0],
        ],
        atol=1e-6,
    )
    assert len(result["origins"]) == 1
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected_lorg_vertices = [2] * 11 + [1] * 6 + [0] * 12 + [2]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 28:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 1)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.733804, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.768634, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        offsets=[[0.18, 0.25, 0.32]],
        end_type="round",
    ) == result


def test_offset_contours_with_origins_matches_meshlib_open_variable_zig_round_end_index_map_contract() -> None:
    result = offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (0.2, 0.4), (1.2, 0.8)]],
        offsets=[[0.18, 0.25, 0.32, 0.18]],
        end_type="round",
    )

    assert len(result["contours"]) == 1
    assert len(result["contours"][0]) == 40
    np.testing.assert_allclose(
        result["contours"][0],
        [
            [-0.06202, 0.183081, 0.0],
            [-0.140014, 0.271892, 0.0],
            [-0.177165, 0.371316, 0.0],
            [-0.173549, 0.47245, 0.0],
            [-0.12924, 0.566395, 0.0],
            [-0.044314, 0.644249, 0.0],
            [0.081155, 0.697113, 0.0],
            [1.13315, 0.967126, 0.0],
            [1.206807, 0.977635, 0.0],
            [1.270104, 0.970788, 0.0],
            [1.321903, 0.949431, 0.0],
            [1.361064, 0.916412, 0.0],
            [1.386448, 0.874579, 0.0],
            [1.396917, 0.82678, 0.0],
            [1.39133, 0.775862, 0.0],
            [1.36855, 0.724674, 0.0],
            [1.327436, 0.676062, 0.0],
            [1.26685, 0.632874, 0.0],
            [0.833918, 0.390841, 0.0],
            [1.111803, 0.223607, 0.0],
            [1.198713, 0.155018, 0.0],
            [1.254721, 0.076931, 0.0],
            [1.280866, -0.004564, 0.0],
            [1.278187, -0.083377, 0.0],
            [1.247722, -0.153417, 0.0],
            [1.19051, -0.208594, 0.0],
            [1.10759, -0.242819, 0.0],
            [1.0, -0.25, 0.0],
            [0.0, -0.18, 0.0],
            [-0.072722, -0.165848, 0.0],
            [-0.129284, -0.13713, 0.0],
            [-0.169685, -0.097489, 0.0],
            [-0.193925, -0.050565, 0.0],
            [-0.202006, 0.0, 0.0],
            [-0.193925, 0.050565, 0.0],
            [-0.169685, 0.097489, 0.0],
            [-0.129284, 0.13713, 0.0],
            [-0.072722, 0.165848, 0.0],
            [-0.04253, 0.171723, 0.0],
            [-0.06202, 0.183081, 0.0],
        ],
        atol=1e-6,
    )
    assert len(result["origins"][0]) == len(result["contours"][0])
    expected_lorg_vertices = [2] * 7 + [3] * 11 + [2] + [1] * 9 + [0] * 11 + [2]
    for index, (origin, expected_vert) in enumerate(zip(result["origins"][0], expected_lorg_vertices)):
        assert (origin["l_org"]["contour_id"], origin["l_org"]["vert_id"]) == (0, expected_vert)
        if index == 18:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 3)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 1)
            assert origin["l_ratio"] == pytest.approx(0.543323, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.638497, abs=1e-6)
            assert origin["is_intersection"] is True
        elif index == 38:
            assert (origin["l_dest"]["contour_id"], origin["l_dest"]["vert_id"]) == (0, 0)
            assert (origin["u_org"]["contour_id"], origin["u_org"]["vert_id"]) == (0, 2)
            assert (origin["u_dest"]["contour_id"], origin["u_dest"]["vert_id"]) == (0, 2)
            assert origin["l_ratio"] == pytest.approx(0.415170, abs=1e-6)
            assert origin["u_ratio"] == pytest.approx(0.163901, abs=1e-6)
            assert origin["is_intersection"] is True
        else:
            assert origin["is_intersection"] is False
    assert GeometrySDK().offset_contours_with_origins(
        [[(0.0, 0.0), (1.0, 0.0), (0.2, 0.4), (1.2, 0.8)]],
        offsets=[[0.18, 0.25, 0.32, 0.18]],
        end_type="round",
    ) == result


def test_offset_contours_variable_rejects_single_point_contours() -> None:
    with pytest.raises(ValueError, match="requires closed contours"):
        offset_contours([[(0.0, 0.0)]], offsets=[[0.25]], end_type="cut")


def test_object_lines_pts_roundtrips_meshlib_polyline_blocks() -> None:
    lines = object_lines_from_contours(
        [
            [(0.0, 0.0), (1.25, 0.0), (1.25, 1.5), (0.0, 0.0)],
            [(2.0, -1.0, 0.5), (3.0, -1.0, 0.5)],
        ],
    )

    source = object_lines_to_pts(lines)

    assert source == (
        "BEGIN_Polyline\n"
        "0 0 0\n"
        "1.25 0 0\n"
        "1.25 1.5 0\n"
        "0 0 0\n"
        "END_Polyline\n"
        "BEGIN_Polyline\n"
        "2 -1 0.5\n"
        "3 -1 0.5\n"
        "END_Polyline\n"
    )
    assert object_lines_to_contours(object_lines_from_pts(source)) == object_lines_to_contours(lines)
    assert GeometrySDK().object_lines_to_pts(lines) == source


def test_object_lines_pts_import_accepts_meshlib_trailing_point_fields() -> None:
    source = (
        "BEGIN_Polyline\n"
        "0 0 0 0.75 255 128 64\n"
        "1.25 0 0 0.5 12 34 56\n"
        "1.25 1.5 0 ignored trailing tokens\n"
        "END_Polyline\n"
    )

    lines = object_lines_from_pts(source)

    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 0.0), (1.25, 0.0, 0.0), (1.25, 1.5, 0.0)]
    ]
    assert object_lines_to_contours(GeometrySDK().object_lines_from_pts(source)) == object_lines_to_contours(lines)


def test_object_lines_pts_import_accepts_meshlib_last_coordinate_prefix_suffix() -> None:
    source = (
        "BEGIN_Polyline\n"
        "0 0 3.5mm\n"
        "1 2 1e+2suffix trailing tokens\n"
        "END_Polyline\n"
    )

    lines = object_lines_from_pts(source)

    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 3.5), (1.0, 2.0, 100.0)]
    ]
    assert object_lines_to_contours(GeometrySDK().object_lines_from_pts(source)) == object_lines_to_contours(lines)


def test_object_lines_pts_import_rejects_meshlib_nonlast_coordinate_suffixes() -> None:
    for source in (
        "BEGIN_Polyline\n1x 2 3\nEND_Polyline\n",
        "BEGIN_Polyline\n1 2y 3\nEND_Polyline\n",
    ):
        with pytest.raises(ValueError):
            object_lines_from_pts(source)
        with pytest.raises(ValueError):
            GeometrySDK().object_lines_from_pts(source)


def test_object_lines_dxf_export_matches_meshlib_polyline_entities() -> None:
    lines = object_lines_from_contours(
        [[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]],
    )

    source = object_lines_to_dxf(lines)

    assert source.startswith("0\nSECTION\n2\nENTITIES\n")
    assert "0\nPOLYLINE\n8\n0\n66\n1\n70\n9\n" in source
    assert "0\nVERTEX\n8\n0\n70\n32\n10\n1\n20\n0\n30\n0\n" in source
    assert source.endswith("0\nENDSEC\n0\nEOF\n")


def test_object_lines_mrlines_roundtrips_meshlib_binary_topology() -> None:
    lines = object_lines_from_contours([[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]])

    payload = object_lines_to_mrlines(lines)

    expected = bytearray()
    expected.extend((2).to_bytes(4, "little"))
    for value in (0, 0, 1, 1):
        expected.extend(int(value).to_bytes(4, "little", signed=True))
    expected.extend((2).to_bytes(4, "little"))
    for value in (0, 1):
        expected.extend(int(value).to_bytes(4, "little", signed=True))
    expected.extend((3).to_bytes(4, "little"))
    expected.extend((2).to_bytes(4, "little"))
    expected.extend(np.array([0.0, 0.0, 0.0, 1.0, 2.0, 3.0], dtype="<f4").tobytes())
    assert payload == bytes(expected)
    assert object_lines_to_contours(object_lines_from_mrlines(payload)) == [
        [(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]
    ]


def test_object_lines_ply_roundtrips_meshlib_binary_edges() -> None:
    lines = object_lines_from_contours([[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]])

    payload = object_lines_to_ply(lines)

    expected = bytearray(
        b"ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n"
        b"element vertex 2\nproperty float x\nproperty float y\nproperty float z\n"
        b"element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    expected.extend(np.array([0.0, 0.0, 0.0, 1.0, 2.0, 3.0], dtype="<f4").tobytes())
    expected.extend(int(0).to_bytes(4, "little", signed=True))
    expected.extend(int(1).to_bytes(4, "little", signed=True))
    assert payload == bytes(expected)
    assert object_lines_to_contours(object_lines_from_ply(payload)) == [
        [(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]
    ]


def test_object_lines_ascii_ply_import_matches_meshlib_vertex_edge_loader() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment ascii line fixture\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0 1\n"
        "1 2\n"
    )

    assert object_lines_to_contours(object_lines_from_ply(source.encode())) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_object_lines_svg_import_matches_meshlib_line_and_polyline_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<line x1="1" y1="2" x2="4" y2="6" />'
        '<polyline points="0,0 2,0 2,2" />'
        "</svg>"
    )

    assert hasattr(distance_map_sdk, "object_lines_from_svg")
    lines = distance_map_sdk.object_lines_from_svg(svg)

    assert object_lines_to_contours(lines) == [
        [(1.0, -2.0, 0.0), (4.0, -6.0, 0.0)],
        [(0.0, -0.0, 0.0), (2.0, -0.0, 0.0), (2.0, -2.0, 0.0)],
    ]
    assert lines.metadata["source"] == "MeshLib SVG ObjectLines"


def test_geometry_sdk_object_lines_from_svg_facade_exposes_meshlib_loader() -> None:
    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<line x1="1" y1="2" x2="4" y2="6" />'
        "</svg>"
    )

    lines = GeometrySDK().object_lines_from_svg(svg)

    assert object_lines_to_contours(lines) == [[(1.0, -2.0, 0.0), (4.0, -6.0, 0.0)]]
    assert lines.metadata["source"] == "MeshLib SVG ObjectLines"


def test_object_lines_svg_import_matches_meshlib_polygon_and_rect_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<polygon points="0,0 2,0 2,2" />'
        '<rect x="1" y="2" width="3" height="4" />'
        "</svg>"
    )

    lines = distance_map_sdk.object_lines_from_svg(svg)

    assert object_lines_to_contours(lines) == [
        [(0.0, -0.0, 0.0), (2.0, -0.0, 0.0), (2.0, -2.0, 0.0), (0.0, -0.0, 0.0)],
        [(1.0, -2.0, 0.0), (1.0, -6.0, 0.0), (4.0, -6.0, 0.0), (4.0, -2.0, 0.0), (1.0, -2.0, 0.0)],
    ]


def test_object_lines_svg_import_accepts_meshlib_compact_signed_points_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<polyline points="0,0 10-10 20,0" />'
        '<polygon points="0,0 2-2 4,0" />'
        "</svg>"
    )

    lines = distance_map_sdk.object_lines_from_svg(svg)

    assert object_lines_to_contours(lines) == [
        [(0.0, -0.0, 0.0), (10.0, 10.0, 0.0), (20.0, -0.0, 0.0)],
        [(0.0, -0.0, 0.0), (2.0, 2.0, 0.0), (4.0, -0.0, 0.0), (0.0, -0.0, 0.0)],
    ]
    assert object_lines_to_contours(GeometrySDK().object_lines_from_svg(svg)) == object_lines_to_contours(lines)


def test_object_lines_svg_import_matches_meshlib_circle_and_ellipse_sampling_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<circle cx="1" cy="2" r="3" />'
        '<ellipse cx="-1" cy="4" rx="2" ry="1" />'
        "</svg>"
    )

    contours = object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg))

    assert len(contours) == 2
    assert len(contours[0]) == 33
    assert len(contours[1]) == 33

    assert contours[0][0] == pytest.approx((4.0, -2.0, 0.0), abs=1e-9)
    assert contours[0][8] == pytest.approx((1.0, -5.0, 0.0), abs=1e-9)
    assert contours[0][16] == pytest.approx((-2.0, -2.0, 0.0), abs=1e-9)
    assert contours[0][24] == pytest.approx((1.0, 1.0, 0.0), abs=1e-9)
    assert contours[0][32] == pytest.approx((4.0, -2.0, 0.0), abs=1e-9)

    assert contours[1][0] == pytest.approx((1.0, -4.0, 0.0), abs=1e-9)
    assert contours[1][8] == pytest.approx((-1.0, -5.0, 0.0), abs=1e-9)
    assert contours[1][16] == pytest.approx((-3.0, -4.0, 0.0), abs=1e-9)
    assert contours[1][24] == pytest.approx((-1.0, -3.0, 0.0), abs=1e-9)
    assert contours[1][32] == pytest.approx((1.0, -4.0, 0.0), abs=1e-9)


def test_object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<rect x="1" y="2" width="6" height="4" rx="2" ry="1" />'
        "</svg>"
    )

    contours = object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg))

    assert len(contours) == 1
    assert len(contours[0]) == 133
    assert contours[0][0] == pytest.approx((5.0, -2.0, 0.0), abs=1e-9)
    assert contours[0][16] == pytest.approx((5.0 + 2**0.5, -3.0 + (2**0.5 / 2.0), 0.0), abs=1e-9)
    assert contours[0][32] == pytest.approx((7.0, -3.0, 0.0), abs=1e-9)
    assert contours[0][33] == pytest.approx((7.0, -5.0, 0.0), abs=1e-9)
    assert contours[0][65] == pytest.approx((5.0, -6.0, 0.0), abs=1e-9)
    assert contours[0][66] == pytest.approx((3.0, -6.0, 0.0), abs=1e-9)
    assert contours[0][98] == pytest.approx((1.0, -5.0, 0.0), abs=1e-9)
    assert contours[0][99] == pytest.approx((1.0, -3.0, 0.0), abs=1e-9)
    assert contours[0][131] == pytest.approx((3.0, -2.0, 0.0), abs=1e-9)
    assert contours[0][132] == pytest.approx((5.0, -2.0, 0.0), abs=1e-9)


def test_object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<path d="M 0 0 L 2 0 H 3 V 2 h -1 v 1 z M 10 0 l 0 2 2 0 z m 8 0 0 2 2 0 z" />'
        "</svg>"
    )

    assert object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg)) == [
        [
            (0.0, -0.0, 0.0),
            (2.0, -0.0, 0.0),
            (3.0, -0.0, 0.0),
            (3.0, -2.0, 0.0),
            (2.0, -2.0, 0.0),
            (2.0, -3.0, 0.0),
            (0.0, -0.0, 0.0),
        ],
        [
            (10.0, -0.0, 0.0),
            (10.0, -2.0, 0.0),
            (12.0, -2.0, 0.0),
            (10.0, -0.0, 0.0),
        ],
        [
            (18.0, -0.0, 0.0),
            (18.0, -2.0, 0.0),
            (20.0, -2.0, 0.0),
            (18.0, -0.0, 0.0),
        ],
    ]


def test_object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<path d="M 0 0 C 0 32 32 32 32 0 S 64 -32 64 0 Q 64 32 96 32 T 128 0" />'
        "</svg>"
    )

    contours = object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg))

    assert len(contours) == 1
    assert len(contours[0]) == 129
    assert contours[0][0] == pytest.approx((0.0, -0.0, 0.0), abs=1e-9)
    assert contours[0][16] == pytest.approx((16.0, -24.0, 0.0), abs=1e-9)
    assert contours[0][32] == pytest.approx((32.0, -0.0, 0.0), abs=1e-9)
    assert contours[0][48] == pytest.approx((48.0, 24.0, 0.0), abs=1e-9)
    assert contours[0][64] == pytest.approx((64.0, -0.0, 0.0), abs=1e-9)
    assert contours[0][80] == pytest.approx((72.0, -24.0, 0.0), abs=1e-9)
    assert contours[0][96] == pytest.approx((96.0, -32.0, 0.0), abs=1e-9)
    assert contours[0][112] == pytest.approx((120.0, -24.0, 0.0), abs=1e-9)
    assert contours[0][128] == pytest.approx((128.0, -0.0, 0.0), abs=1e-9)


def test_object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<path d="M 0 0 A 10 10 0 0 1 20 0" />'
        "</svg>"
    )

    contours = object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg))

    assert len(contours) == 1
    assert len(contours[0]) == 33
    assert contours[0][0] == pytest.approx((0.0, -0.0, 0.0), abs=1e-9)
    assert contours[0][16] == pytest.approx((10.0, 10.0, 0.0), abs=1e-9)
    assert contours[0][32] == pytest.approx((20.0, -0.0, 0.0), abs=1e-9)


def test_object_lines_svg_import_matches_meshlib_transform_attributes_y_flip() -> None:
    from geometry_sdk import distance_map as distance_map_sdk

    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<g transform="translate(10, 20)">'
        '<line x1="1" y1="2" x2="3" y2="4" transform="scale(2)" />'
        "</g>"
        '<line x1="1" y1="2" x2="3" y2="4" transform="matrix(1 2 3 4 5 6)" />'
        '<line x1="2" y1="1" x2="2" y2="2" transform="rotate(90, 1, 1)" />'
        '<line x1="1" y1="2" x2="3" y2="4" transform="skewX(45)" />'
        '<line x1="1" y1="2" x2="3" y2="4" transform="skewY(45)" />'
        "</svg>"
    )

    contours = object_lines_to_contours(distance_map_sdk.object_lines_from_svg(svg))
    expected = [
        [(12.0, -24.0, 0.0), (16.0, -28.0, 0.0)],
        [(12.0, -16.0, 0.0), (20.0, -28.0, 0.0)],
        [(1.0, -2.0, 0.0), (0.0, -2.0, 0.0)],
        [(3.0, -2.0, 0.0), (7.0, -4.0, 0.0)],
        [(1.0, -3.0, 0.0), (3.0, -7.0, 0.0)],
    ]
    assert len(contours) == len(expected)
    for actual_contour, expected_contour in zip(contours, expected):
        assert len(actual_contour) == len(expected_contour)
        for actual, expected_point in zip(actual_contour, expected_contour):
            assert actual == pytest.approx(expected_point, abs=1e-9)


def test_object_lines_ascii_ply_import_preserves_meshlib_uv_and_texture_comment() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment TextureFile brushed-metal.jpg\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property float s\n"
        "property float t\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 0.25 0.75\n"
        "1 0 0 0.5 0.125\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert object_lines_to_contours(lines) == [[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)]]
    assert lines.metadata["uv_coords"] == [[0.25, 0.75], [0.5, 0.125]]
    assert lines.metadata["texture_files"] == ["brushed-metal.jpg"]


def test_object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment TextureFile brushed-metal.jpg   \t\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property float s\n"
        "property float t\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 0.25 0.75\n"
        "1 0 0 0.5 0.125\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.metadata["uv_coords"] == [[0.25, 0.75], [0.5, 0.125]]
    assert lines.metadata["texture_files"] == ["brushed-metal.jpg"]


def test_object_lines_ascii_ply_import_accepts_meshlib_format_version_tuple() -> None:
    source = (
        "ply\n"
        "format ascii 1.1\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_magic() -> None:
    source = (
        "ply   \n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_accepts_meshlib_trailing_format_line_tokens() -> None:
    source = (
        "ply\n"
        "format ascii 1.0 generated-by-tool\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_accepts_meshlib_format_minor_prefix_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_alpha_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0alpha\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_underscore_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0_alpha\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_accepts_meshlib_trailing_element_line_tokens() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2 generated-by-tool\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1 generated-by-tool\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_rejects_meshlib_element_count_alpha_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2vertices\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1edges\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_rejects_meshlib_element_count_underscore_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2_vertices\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1_edges\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_accepts_meshlib_trailing_property_line_tokens() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x generated-by-tool\n"
        "property float y generated-by-tool\n"
        "property float z generated-by-tool\n"
        "element face 1\n"
        "property list uchar int vertex_indices generated-by-tool\n"
        "element edge 1\n"
        "property int vertex1 generated-by-tool\n"
        "property int vertex2 generated-by-tool\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "2 0 1\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_rejects_leading_header_keyword_whitespace_like_meshlib() -> None:
    base_lines = [
        "ply",
        "format ascii 1.0",
        "element vertex 2",
        "property float x",
        "property float y",
        "property float z",
        "element edge 1",
        "property int vertex1",
        "property int vertex2",
        "end_header",
        "0 0 0",
        "1 0 0",
        "0 1",
    ]
    for line_index, line in [
        (1, " format ascii 1.0"),
        (2, " element vertex 2"),
        (3, " property float x"),
    ]:
        lines = list(base_lines)
        lines[line_index] = line
        source = "\n".join(lines) + "\n"

        with pytest.raises(ValueError):
            object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_accepts_meshlib_spaced_format_version_tuple() -> None:
    source = (
        "ply\n"
        "format ascii 1 . 0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_end_header() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header \n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_ascii_ply_import_rejects_unknown_header_directives_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "made_up_header value\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_accepts_vertex_only_files_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment vertex only line fixture\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
    )

    lines = object_lines_from_ply(source.encode())

    np.testing.assert_allclose(
        lines.points,
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
    )
    assert lines.lines.tolist() == []
    assert object_lines_to_contours(lines) == []


def test_object_lines_ascii_ply_import_casts_coordinates_to_vector3f_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property double x\n"
        "property double y\n"
        "property double z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0.123456789123 100000000.25 0.000000123456789\n"
        "1.987654321987 -100000000.25 3.141592653589793\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    np.testing.assert_array_equal(
        lines.points,
        np.asarray(
            [
                [0.12345679104328156, 100000000.0, 0.0000001234567861274627],
                [1.9876543283462524, -100000000.0, 3.1415927410125732],
            ],
            dtype=np.float64,
        ),
    )


def test_object_lines_ascii_ply_import_wraps_narrow_vertex_coordinates_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property char x\n"
        "property short y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "257 65537 0\n"
        "0 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    np.testing.assert_array_equal(lines.points, np.asarray([[1.0, 1.0, 0.0], [0.0, 0.0, 0.0]], dtype=np.float64))
    assert lines.lines.tolist() == [[0, 1]]


def test_object_lines_ascii_ply_import_preserves_meshlib_vertex_colors() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment ascii colored line fixture\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property uchar red\n"
        "property uchar green\n"
        "property uchar blue\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 255 0 0\n"
        "1 0 0 0 255 0\n"
        "1 1 0 0 0 255\n"
        "0 1\n"
        "1 2\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.coloring_type == "PerVertex"
    assert lines.vert_colors == [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_object_lines_ply_import_prefers_meshlib_rgb_short_names_over_long_color_names() -> None:
    ascii_source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 1\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property uchar r\n"
        "property uchar g\n"
        "property uchar b\n"
        "property uchar red\n"
        "property uchar green\n"
        "property uchar blue\n"
        "end_header\n"
        "0 0 0 1 2 3 200 201 202\n"
    )
    ascii_lines = object_lines_from_ply(ascii_source.encode())
    assert ascii_lines.vert_colors == [[1, 2, 3, 255]]

    binary_payload = bytearray(
        b"ply\nformat binary_little_endian 1.0\n"
        b"element vertex 1\nproperty float x\nproperty float y\nproperty float z\n"
        b"property uchar r\nproperty uchar g\nproperty uchar b\n"
        b"property uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n"
    )
    binary_payload.extend(np.asarray([0.0, 0.0, 0.0], dtype="<f4").tobytes())
    binary_payload.extend([1, 2, 3, 200, 201, 202])
    binary_lines = object_lines_from_ply(bytes(binary_payload))

    assert binary_lines.vert_colors == ascii_lines.vert_colors


def test_object_lines_ascii_ply_import_casts_float_vertex_colors_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment ascii float-colored line fixture\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property float red\n"
        "property float green\n"
        "property float blue\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 1.0 0.0 0.0\n"
        "1 0 0 0.0 0.5 1.0\n"
        "1 1 0 255.9 128.2 2.8\n"
        "0 1\n"
        "1 2\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.coloring_type == "PerVertex"
    assert lines.vert_colors == [[1, 0, 0, 255], [0, 0, 1, 255], [255, 128, 2, 255]]
    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_object_lines_ascii_ply_import_wraps_integer_vertex_colors_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property int red\n"
        "property int green\n"
        "property int blue\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 -1 256 300\n"
        "1 0 0 -255 -256 -257\n"
        "1 1 0 511 512 513\n"
        "0 1\n"
        "1 2\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.coloring_type == "PerVertex"
    assert lines.vert_colors == [[255, 0, 44, 255], [1, 0, 255, 255], [255, 0, 1, 255]]
    assert object_lines_to_contours(lines) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_object_lines_ascii_ply_import_ignores_unneeded_list_properties_like_meshlib() -> None:
    with_vertex_list_before_xyz = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property list uchar int adjacent_vertices\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "2 1 2 0 0 0\n"
        "1 0 1 0 0\n"
        "0 1 1 0\n"
        "0 1\n"
        "1 2\n"
    )
    with_vertex_list_after_xyz = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property list uchar int adjacent_vertices\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0 2 1 2\n"
        "1 0 0 1 0\n"
        "1 1 0 0\n"
        "0 1\n"
        "1 2\n"
    )
    with_edge_list = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "property list uchar float weights\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0 1 2 0.5 0.25\n"
        "1 2 0\n"
    )

    for source in [with_vertex_list_before_xyz, with_vertex_list_after_xyz, with_edge_list]:
        assert object_lines_to_contours(object_lines_from_ply(source.encode())) == [
            [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
        ]


def test_object_lines_ascii_ply_import_accepts_meshlib_property_name_prefix_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float bad-name\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "99 0 0 0\n"
        "99 1 0 0\n"
        "0 1\n"
    )

    assert object_lines_to_contours(object_lines_from_ply(source.encode())) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)]
    ]


def test_object_lines_ascii_ply_import_rejects_non_identifier_property_names_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float 1bad\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "99 0 0 0\n"
        "99 1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ascii_ply_import_accepts_meshlib_last_integer_prefix_suffix() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1.9\n"
    )

    assert object_lines_from_ply(source.encode()).lines.tolist() == [[0, 1]]


def test_object_lines_ascii_ply_import_skips_meshlib_unsigned_negative_edge_endpoint() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property uint vertex1\n"
        "property uint vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 -1\n"
    )

    assert object_lines_from_ply(source.encode()).lines.tolist() == []


def test_object_lines_ascii_ply_import_rejects_float64_type_alias_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float64 x\n"
        "property float64 y\n"
        "property float64 z\n"
        "element edge 1\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    with pytest.raises(ValueError, match="unsupported .PLY file with polylines"):
        object_lines_from_ply(source.encode())


def test_object_lines_ply_export_writes_meshlib_vertex_colors() -> None:
    lines = ObjectLinesDocument(
        points=np.asarray([[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]], dtype=np.float64),
        lines=np.asarray([[0, 1]], dtype=np.int64),
        coloring_type="PerVertex",
        vert_colors=[[255, 0, 0, 255], [0, 127, 255, 255]],
    )

    payload = object_lines_to_ply(lines)

    expected = bytearray(
        b"ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n"
        b"element vertex 2\nproperty float x\nproperty float y\nproperty float z\n"
        b"property uchar red\nproperty uchar green\nproperty uchar blue\n"
        b"element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    for point, color in [
        ([0.0, 0.0, 0.0], [255, 0, 0]),
        ([1.0, 2.0, 3.0], [0, 127, 255]),
    ]:
        expected.extend(np.asarray(point, dtype="<f4").tobytes())
        expected.extend(color)
    expected.extend(int(0).to_bytes(4, "little", signed=True))
    expected.extend(int(1).to_bytes(4, "little", signed=True))
    assert payload == bytes(expected)
    assert object_lines_from_ply(payload).vert_colors == lines.vert_colors


def test_object_lines_ascii_ply_import_skips_mesh_face_elements_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "comment ascii mesh and line fixture\n"
        "element vertex 4\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element face 1\n"
        "property list uchar int vertex_indices\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0 1 0\n"
        "3 0 1 2\n"
        "0 3\n"
        "3 2\n"
    )

    assert object_lines_to_contours(object_lines_from_ply(source.encode())) == [
        [(0.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_object_lines_ascii_ply_import_ignores_invalid_edges_like_meshlib() -> None:
    self_loop = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0 0\n"
        "0 1\n"
    )
    out_of_range = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0 3\n"
        "0 1\n"
    )
    negative = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "-1 1\n"
        "0 1\n"
    )

    for source in [self_loop, out_of_range, negative]:
        assert object_lines_from_ply(source.encode()).lines.tolist() == [[0, 1]]

    over_degree = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 4\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 3\n"
        "property int vertex1\n"
        "property int vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1 0\n"
        "0 -1 0\n"
        "0 1\n"
        "0 2\n"
        "0 3\n"
    )

    assert object_lines_from_ply(over_degree.encode()).lines.tolist() == [[0, 1], [0, 2]]


def test_object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property int source\n"
        "property int target\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 1\n"
    )

    lines = object_lines_from_ply(source.encode())

    assert lines.points.tolist() == [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
    assert lines.lines.tolist() == []


def test_object_lines_ascii_ply_import_casts_float_edge_indices_like_meshlib() -> None:
    source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 2\n"
        "property float vertex1\n"
        "property float vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "1 1 0\n"
        "0.9 1.2\n"
        "1.8 2.9\n"
    )

    assert object_lines_from_ply(source.encode()).lines.tolist() == [[0, 1], [1, 2]]


def test_object_lines_ascii_ply_import_wraps_narrow_edge_indices_like_meshlib() -> None:
    char_source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property char vertex1\n"
        "property char vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 257\n"
    )
    assert object_lines_from_ply(char_source.encode()).lines.tolist() == [[0, 1]]

    short_source = (
        "ply\n"
        "format ascii 1.0\n"
        "element vertex 2\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "element edge 1\n"
        "property short vertex1\n"
        "property short vertex2\n"
        "end_header\n"
        "0 0 0\n"
        "1 0 0\n"
        "0 65537\n"
    )
    assert object_lines_from_ply(short_source.encode()).lines.tolist() == [[0, 1]]


def test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_unneeded_vertex_property() -> None:
    payload = bytearray(
        b"ply\nformat binary_little_endian 1.0\n"
        b"element vertex 2\nproperty list float int ghost\nproperty float x\nproperty float y\nproperty float z\n"
        b"element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    for point in [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)]:
        payload.extend(np.array([1.9], dtype="<f4").tobytes())
        payload.extend(int(99).to_bytes(4, "little", signed=True))
        payload.extend(np.array(point, dtype="<f4").tobytes())
    payload.extend(int(0).to_bytes(4, "little", signed=True))
    payload.extend(int(1).to_bytes(4, "little", signed=True))

    lines = object_lines_from_ply(bytes(payload))

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_skipped_element() -> None:
    payload = bytearray(
        b"ply\nformat binary_little_endian 1.0\n"
        b"element vertex 2\nproperty float x\nproperty float y\nproperty float z\n"
        b"element ghost 1\nproperty list float int payload\n"
        b"element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    payload.extend(np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0], dtype="<f4").tobytes())
    payload.extend(np.array([1.9], dtype="<f4").tobytes())
    payload.extend(int(7).to_bytes(4, "little", signed=True))
    payload.extend(int(0).to_bytes(4, "little", signed=True))
    payload.extend(int(1).to_bytes(4, "little", signed=True))

    lines = object_lines_from_ply(bytes(payload))

    assert lines.lines.tolist() == [[0, 1]]
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties() -> None:
    payload = bytearray(
        b"ply\nformat binary_little_endian 1.0\n"
        b"element vertex 2\nproperty float x\nproperty float y\nproperty float z\n"
        b"element edge 1\nproperty int source\nproperty int target\nend_header\n"
    )
    payload.extend(np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0], dtype="<f4").tobytes())
    payload.extend(int(0).to_bytes(4, "little", signed=True))
    payload.extend(int(1).to_bytes(4, "little", signed=True))

    lines = object_lines_from_ply(bytes(payload))

    assert lines.lines.tolist() == []
    np.testing.assert_array_equal(lines.points, np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64))


def test_object_lines_binary_big_endian_ply_import_matches_meshlib_vertex_edge_loader() -> None:
    payload = bytearray(
        b"ply\nformat binary_big_endian 1.0\ncomment big endian line fixture\n"
        b"element vertex 3\nproperty float x\nproperty float y\nproperty float z\n"
        b"element edge 2\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    payload.extend(np.array([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0], dtype=">f4").tobytes())
    for value in [0, 1, 1, 2]:
        payload.extend(int(value).to_bytes(4, "big", signed=True))

    assert object_lines_to_contours(object_lines_from_ply(bytes(payload))) == [
        [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]
    ]


def test_geometry_sdk_exposes_contour_distance_map() -> None:
    distance_map = GeometrySDK().distance_map_from_contours(
        SQUARE_CONTOUR,
        width=3,
        height=3,
        origin=(0.0, 0.0),
        pixel_size=1.0,
        signed=True,
    )

    assert distance_map.values[1, 1] == pytest.approx(-0.5)


def test_distance_map_from_mesh_matches_meshlib_pixel_center_rays() -> None:
    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 2.0],
                [2.0, 0.0, 2.0],
                [2.0, 2.0, 2.0],
                [0.0, 2.0, 2.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )

    distance_map = distance_map_from_mesh(
        mesh,
        width=2,
        height=2,
        origin=(0.0, 0.0, 0.0),
        x_range=(2.0, 0.0, 0.0),
        y_range=(0.0, 2.0, 0.0),
        direction=(0.0, 0.0, 1.0),
    )

    assert isinstance(distance_map, DistanceMapDocument)
    assert distance_map.valid_count == 4
    assert distance_map.origin == (0.0, 0.0)
    assert distance_map.pixel_size == (1.0, 1.0)
    np.testing.assert_allclose(distance_map.values, [[2.0, 2.0], [2.0, 2.0]])


def test_geometry_sdk_exposes_mesh_distance_map() -> None:
    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 2.0],
                [2.0, 0.0, 2.0],
                [2.0, 2.0, 2.0],
                [0.0, 2.0, 2.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )

    distance_map = GeometrySDK().distance_map_from_mesh(
        mesh,
        width=2,
        height=2,
        origin=(0.0, 0.0, 0.0),
        x_range=(2.0, 0.0, 0.0),
        y_range=(0.0, 2.0, 0.0),
        direction=(0.0, 0.0, 1.0),
    )

    assert distance_map.values[0, 0] == pytest.approx(2.0)


def test_distance_map_from_tiff_matches_meshlib_scalar_float_import(tmp_path) -> None:
    path = tmp_path / "height-field.tiff"
    source = np.array(
        [
            [1.25, -2.5, 3.75],
            [4.5, 5.25, -6.5],
        ],
        dtype=np.float32,
    )
    Image.fromarray(source, mode="F").save(path)

    distance_map = distance_map_from_tiff(path)

    assert isinstance(distance_map, DistanceMapDocument)
    assert distance_map.width == 3
    assert distance_map.height == 2
    assert distance_map.origin == (0.0, 0.0)
    assert distance_map.pixel_size == (1.0, 1.0)
    assert distance_map.valid_count == 6
    assert distance_map.min_value == pytest.approx(-6.5)
    assert distance_map.max_value == pytest.approx(5.25)
    assert distance_map.metadata["source"] == "MeshLib-style TIFF distance-map import"
    np.testing.assert_allclose(distance_map.values, source)


def test_geometry_sdk_exposes_tiff_distance_map_import(tmp_path) -> None:
    path = tmp_path / "sdk-height-field.tif"
    Image.fromarray(np.array([[7.5, -1.25]], dtype=np.float32), mode="F").save(path)

    distance_map = GeometrySDK().distance_map_from_tiff(path)

    assert distance_map.width == 2
    assert distance_map.height == 1
    assert distance_map.values[0, 1] == pytest.approx(-1.25)


def test_distance_map_to_tiff_roundtrips_meshlib_scalar_float_export(tmp_path) -> None:
    invalid = np.finfo(np.float32).min
    path = tmp_path / "exported-height-field.tiff"
    distance_map = DistanceMapDocument(
        width=3,
        height=2,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        values=np.array([[1.25, invalid, 3.75], [4.5, 5.25, -6.5]], dtype=np.float32),
        valid_count=5,
        min_value=-6.5,
        max_value=5.25,
    )

    exported_path = distance_map_to_tiff(distance_map, path)
    reloaded = distance_map_from_tiff(exported_path)

    assert exported_path == path
    assert reloaded.width == distance_map.width
    assert reloaded.height == distance_map.height
    assert reloaded.valid_count == distance_map.valid_count
    np.testing.assert_allclose(reloaded.values, distance_map.values)


def test_distance_map_to_tiff_preserves_meshlib_transform_metadata(tmp_path) -> None:
    path = tmp_path / "exported-transformed-height-field.tiff"
    distance_map = DistanceMapDocument(
        width=2,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(2.5, 4.0),
        values=np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32),
        valid_count=4,
        min_value=1.0,
        max_value=4.0,
    )

    exported_path = distance_map_to_tiff(distance_map, path)
    reloaded = distance_map_from_tiff(exported_path)

    assert reloaded.origin == distance_map.origin
    assert reloaded.pixel_size == distance_map.pixel_size
    np.testing.assert_allclose(reloaded.values, distance_map.values)


def test_distance_map_to_tiff_preserves_explicit_meshlib_model_transform(tmp_path) -> None:
    path = tmp_path / "exported-rotated-height-field.tiff"
    model_transform = (
        0.0,
        -2.0,
        0.0,
        10.0,
        3.0,
        0.0,
        0.5,
        20.0,
        0.0,
        0.0,
        1.25,
        30.0,
        0.0,
        0.0,
        0.0,
        1.0,
    )
    distance_map = DistanceMapDocument(
        width=2,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(3.0, 2.0),
        values=np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32),
        valid_count=4,
        min_value=1.0,
        max_value=4.0,
        model_transform=model_transform,
    )

    exported_path = distance_map_to_tiff(distance_map, path)
    reloaded = distance_map_from_tiff(exported_path)

    assert reloaded.model_transform == model_transform
    np.testing.assert_allclose(reloaded.values, distance_map.values)


def test_geometry_sdk_exposes_tiff_distance_map_export(tmp_path) -> None:
    path = tmp_path / "sdk-exported-height-field.tif"
    distance_map = DistanceMapDocument(
        width=2,
        height=1,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        values=np.array([[7.5, -1.25]], dtype=np.float32),
        valid_count=2,
        min_value=-1.25,
        max_value=7.5,
    )

    exported_path = GeometrySDK().distance_map_to_tiff(distance_map, path)

    assert exported_path == path
    np.testing.assert_allclose(distance_map_from_tiff(path).values, [[7.5, -1.25]])


def test_distance_map_to_iso_segments_matches_meshlib_real_space_transform() -> None:
    distance_map = DistanceMapDocument(
        width=2,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(2.0, 4.0),
        values=np.array([[-1.0, 1.0], [-1.0, 1.0]], dtype=np.float32),
        valid_count=4,
        min_value=-1.0,
        max_value=1.0,
    )

    iso = distance_map_to_iso_segments(distance_map, iso_value=0.0)

    assert isinstance(iso, IsoLineSegmentsDocument)
    assert iso.segment_count == 1
    assert iso.segments.shape == (1, 2, 2)
    np.testing.assert_allclose(iso.segments[0, 0], [12.0, 26.0], atol=1e-6)
    np.testing.assert_allclose(iso.segments[0, 1], [12.0, 22.0], atol=1e-6)


def test_geometry_sdk_exposes_distance_map_iso_segments() -> None:
    distance_map = DistanceMapDocument(
        width=2,
        height=2,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        values=np.array([[-1.0, 1.0], [-1.0, 1.0]], dtype=np.float32),
        valid_count=4,
        min_value=-1.0,
        max_value=1.0,
    )

    iso = GeometrySDK().distance_map_to_iso_segments(distance_map, iso_value=0.0)

    assert iso.segment_count == 1


def test_distance_map_merge_matches_meshlib_invalid_and_extent_contract() -> None:
    invalid = np.finfo(np.float32).min
    left = DistanceMapDocument(
        width=3,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(2.0, 4.0),
        values=np.array([[2.0, invalid, -1.0], [4.0, 8.0, 16.0]], dtype=np.float32),
        valid_count=5,
        min_value=-1.0,
        max_value=16.0,
    )
    right = DistanceMapDocument(
        width=2,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(2.0, 4.0),
        values=np.array([[3.0, 5.0], [invalid, 6.0]], dtype=np.float32),
        valid_count=3,
        min_value=3.0,
        max_value=6.0,
    )

    merged_min = distance_map_merge(left, right, mode="min")
    merged_max = distance_map_merge(left, right, mode="max")
    subtracted = distance_map_merge(left, right, mode="subtract")

    np.testing.assert_allclose(merged_min.values, [[2.0, 5.0, -1.0], [4.0, 6.0, 16.0]])
    np.testing.assert_allclose(merged_max.values, [[3.0, 5.0, -1.0], [4.0, 8.0, 16.0]])
    assert subtracted.values[0, 1] == invalid
    assert subtracted.values[1, 0] == invalid
    np.testing.assert_allclose(
        subtracted.values[[0, 0, 1, 1], [0, 2, 1, 2]],
        [-1.0, -1.0, 2.0, 16.0],
    )
    assert merged_min.valid_count == 6
    assert subtracted.valid_count == 4
    assert merged_min.origin == left.origin
    assert merged_min.pixel_size == left.pixel_size


def test_geometry_sdk_exposes_distance_map_merge() -> None:
    left = DistanceMapDocument(
        width=1,
        height=1,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        values=np.array([[2.0]], dtype=np.float32),
        valid_count=1,
        min_value=2.0,
        max_value=2.0,
    )
    right = DistanceMapDocument(
        width=1,
        height=1,
        origin=(0.0, 0.0),
        pixel_size=(1.0, 1.0),
        values=np.array([[5.0]], dtype=np.float32),
        valid_count=1,
        min_value=5.0,
        max_value=5.0,
    )

    merged = GeometrySDK().distance_map_merge(left, right, mode="max")

    assert merged.values[0, 0] == pytest.approx(5.0)


def test_distance_map_contour_boolean_matches_meshlib_composition() -> None:
    contours_a = [[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]]
    contours_b = [[(1.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 2.0), (1.0, 0.0)]]

    intersection = distance_map_contour_boolean(
        contours_a,
        contours_b,
        mode="intersection",
        width=6,
        height=5,
        origin=(-1.0, -1.0),
        pixel_size=1.0,
    )
    subtract = distance_map_contour_boolean(
        contours_a,
        contours_b,
        mode="subtract",
        width=6,
        height=5,
        origin=(-1.0, -1.0),
        pixel_size=1.0,
    )

    np.testing.assert_allclose(
        intersection.segments,
        [
            [[1.5, 0.0], [1.0, 0.5]],
            [[2.0, 0.5], [1.5, 0.0]],
            [[1.0, 0.5], [1.0, 1.5]],
            [[2.0, 1.5], [2.0, 0.5]],
            [[1.0, 1.5], [1.5, 2.0]],
            [[1.5, 2.0], [2.0, 1.5]],
        ],
    )
    np.testing.assert_allclose(
        subtract.segments,
        [
            [[0.5, 0.0], [0.0, 0.5]],
            [[1.0, 0.5], [0.5, 0.0]],
            [[0.0, 0.5], [0.0, 1.5]],
            [[1.0, 1.5], [1.0, 0.5]],
            [[0.0, 1.5], [0.5, 2.0]],
            [[0.5, 2.0], [1.0, 1.5]],
        ],
    )
    assert intersection.metadata["mode"] == "intersection"
    assert subtract.metadata["mode"] == "subtract"


def test_geometry_sdk_exposes_distance_map_contour_boolean() -> None:
    contours_a = [[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]]
    contours_b = [[(1.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 2.0), (1.0, 0.0)]]

    union = GeometrySDK().distance_map_contour_boolean(
        contours_a,
        contours_b,
        mode="union",
        width=6,
        height=5,
        origin=(-1.0, -1.0),
        pixel_size=1.0,
    )

    assert union.segment_count == 10

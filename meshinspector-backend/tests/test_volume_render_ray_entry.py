"""Regression: volume-render ray entry. A fixed-step ray starting OUTSIDE the
volume must keep stepping until it enters, instead of terminating at the first
out-of-bounds sample (which left small-step rays from outside rendering empty).
"""
from __future__ import annotations

import numpy as np

from geometry_sdk.engine import GeometrySDK

SDK = GeometrySDK()


def _ray(ray_start, sampling_step):
    vol = np.zeros((10, 10, 10), dtype=np.float32)
    vol[3:7, 3:7, 3:7] = 1.0
    return SDK.voxel_volume_render_ray(
        vol.ravel(order="F").tolist(),
        shape=(10, 10, 10),
        voxel_size=(1.0, 1.0, 1.0),
        ray_start=ray_start,
        ray_direction=(0.0, 0.0, 1.0),
        sampling_step=sampling_step,
    )


def test_ray_from_outside_small_step_enters_volume() -> None:
    # z=-1 is outside; step 0.5 lands a sample in the pre-entry zone. The ray must
    # still reach the bright block (was 0 visited before the has_entered fix).
    result = _ray((4.5, 4.5, -1.0), 0.5)
    assert len(result.visited_indices) > 0
    assert len(result.accepted_indices) > 0


def test_ray_from_inside_still_works() -> None:
    result = _ray((4.5, 4.5, 4.5), 0.5)
    assert len(result.visited_indices) > 0

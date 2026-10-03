# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

import math
FIT_FLOOR_ARCSEC = 1.0

def assert_fit_within_floor(name: str, worst_ang_arcsec: float) -> None:
    if not math.isfinite(worst_ang_arcsec) or worst_ang_arcsec > FIT_FLOOR_ARCSEC:
        raise SystemExit(f'{name}: worst fit error {worst_ang_arcsec:.4f}" exceeds the fit error limit of {FIT_FLOOR_ARCSEC}" — refusing to emit this kernel')

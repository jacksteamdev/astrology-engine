# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

from __future__ import annotations
import numpy as np
import requests
from horizons import fetch_states
from jplephem.spk import SPK
ARCSEC_PER_RAD = 206264.806247
GATE_LINE_ARCSEC = 0.9375 * 3600.0

def angular_sep_arcsec(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    na = np.linalg.norm(a, axis=1)
    nb = np.linalg.norm(b, axis=1)
    dot = np.einsum('ij,ij->i', a, b) / (na * nb)
    dot = np.clip(dot, -1.0, 1.0)
    return np.arccos(dot) * ARCSEC_PER_RAD

def validate_spk(spk_path: str, target: int, command: str, jd_start: float, jd_stop: float, n_spots: int, *, get=requests.get) -> dict:
    step_days = (jd_stop - jd_start) / (n_spots - 1)
    jd, pos_hz, _vel = fetch_states(command, jd_start, jd_stop, step_days, get=get)
    if len(jd) < n_spots:
        raise RuntimeError(f'validation grid is short: got {len(jd)} of {n_spots} requested spot epochs for COMMAND={command} — too few samples to trust the gate. Refusing to validate.')
    spk = SPK.open(spk_path)
    try:
        seg = spk[10, target]
        ours = np.empty((len(jd), 3))
        for i, jdi in enumerate(jd):
            ours[i] = seg.compute(jdi)
    finally:
        spk.close()
    resid_km = np.linalg.norm(ours - pos_hz, axis=1)
    ang = angular_sep_arcsec(ours, pos_hz)
    i_km = int(np.argmax(resid_km))
    i_ang = int(np.argmax(ang))
    return {'n_spots': len(jd), 'worst_resid_km': float(resid_km[i_km]), 'worst_resid_km_jd': float(jd[i_km]), 'worst_ang_arcsec': float(ang[i_ang]), 'worst_ang_arcsec_jd': float(jd[i_ang]), 'worst_ang_deg': float(ang[i_ang] / 3600.0), 'gate_line_arcsec': GATE_LINE_ARCSEC, 'margin_factor': float(GATE_LINE_ARCSEC / max(ang[i_ang], 1e-12)), 'mean_resid_km': float(resid_km.mean()), 'mean_ang_arcsec': float(ang.mean())}

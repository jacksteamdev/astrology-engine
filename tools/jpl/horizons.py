# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

from __future__ import annotations
import io
import time
import numpy as np
import requests
HORIZONS = 'https://ssd.jpl.nasa.gov/api/horizons.api'
J2000_JD_TDB = 2451545.0
SEC_PER_DAY = 86400.0

def fetch_states(command: str, jd_start: float, jd_stop: float, step_days: float, *, center: str='500@10', get=requests.get, max_retries: int=4) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    rows_total = int(round((jd_stop - jd_start) / step_days)) + 1
    rows_per_page = 70000
    pages = (rows_total + rows_per_page - 1) // rows_per_page
    jd_all: list[np.ndarray] = []
    pos_all: list[np.ndarray] = []
    vel_all: list[np.ndarray] = []
    for p in range(pages):
        i0 = p * rows_per_page
        i1 = min(i0 + rows_per_page, rows_total)
        start_jd = jd_start + i0 * step_days
        stop_jd = jd_start + (i1 - 1) * step_days
        n_intervals = i1 - 1 - i0
        jd, pos, vel = _fetch_window(get, command, start_jd, stop_jd, n_intervals, center, max_retries)
        jd_all.append(jd)
        pos_all.append(pos)
        vel_all.append(vel)
    jd = np.concatenate(jd_all)
    pos = np.concatenate(pos_all, axis=0)
    vel = np.concatenate(vel_all, axis=0)
    _, uniq = np.unique(np.round(jd, 9), return_index=True)
    uniq.sort()
    n_got = len(uniq)
    if n_got < rows_total:
        raise RuntimeError(f'Horizons returned {n_got} of {rows_total} expected epochs for COMMAND={command} over JD {jd_start:.6f}..{jd_stop:.6f} step {step_days}d — a short fetch would thin the grid and degrade the fit. Refusing to build on partial data.')
    return (jd[uniq], pos[uniq], vel[uniq])

def _fetch_window(get, command: str, start_jd: float, stop_jd: float, n_intervals: int, center: str, max_retries: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    params = {'format': 'text', 'COMMAND': command, 'OBJ_DATA': 'NO', 'MAKE_EPHEM': 'YES', 'EPHEM_TYPE': 'VECTORS', 'CENTER': center, 'REF_PLANE': 'FRAME', 'REF_SYSTEM': 'ICRF', 'VEC_TABLE': '2', 'VEC_LABELS': 'NO', 'VEC_CORR': 'NONE', 'CSV_FORMAT': 'YES', 'OUT_UNITS': 'KM-S', 'TIME_TYPE': 'TDB', 'START_TIME': f'JD{start_jd:.9f}', 'STOP_TIME': f'JD{stop_jd:.9f}', 'STEP_SIZE': str(int(n_intervals))}
    last_err: Exception | None = None
    for attempt in range(max_retries):
        try:
            r = get(HORIZONS, params=params, timeout=180)
            if r.status_code == 200 and '$$SOE' in r.text:
                return _parse_vectors(r.text)
            last_err = RuntimeError(f'Horizons {r.status_code}; no SOE block. Head: {r.text[:400]}')
        except requests.RequestException as e:
            last_err = e
        time.sleep(2.0 * (attempt + 1))
    raise RuntimeError(f'Horizons fetch failed after {max_retries} tries: {last_err}')

def _parse_vectors(text: str) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    body = text.split('$$SOE', 1)[1].split('$$EOE', 1)[0]
    jd, pos, vel = ([], [], [])
    for line in io.StringIO(body):
        line = line.strip()
        if not line:
            continue
        parts = [c.strip() for c in line.split(',') if c.strip() != '']
        jd.append(float(parts[0]))
        pos.append([float(parts[2]), float(parts[3]), float(parts[4])])
        vel.append([float(parts[5]), float(parts[6]), float(parts[7])])
    return (np.asarray(jd), np.asarray(pos), np.asarray(vel))

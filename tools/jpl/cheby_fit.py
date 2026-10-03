from __future__ import annotations
import numpy as np
from horizons import J2000_JD_TDB, SEC_PER_DAY
from spk_writer import Type2Segment

def jd_to_et(jd_tdb: np.ndarray) -> np.ndarray:
    return (jd_tdb - J2000_JD_TDB) * SEC_PER_DAY

def fit_type2_segment(jd: np.ndarray, pos_km: np.ndarray, *, target: int, center: int, frame: int, intlen_days: float, degree: int, segid: str) -> tuple[Type2Segment, dict]:
    et = jd_to_et(jd)
    intlen_sec = intlen_days * SEC_PER_DAY
    t0 = et[0]
    t_end = et[-1]
    span = t_end - t0
    n_records = int(np.floor(span / intlen_sec + 1e-09))
    if n_records < 1:
        raise ValueError('span shorter than one interval — widen the fetch or shrink intlen')
    rsize = 2 + 3 * (degree + 1)
    records: list[list[float]] = []
    worst_resid = 0.0
    worst_jd = float('nan')
    for k in range(n_records):
        lo = t0 + k * intlen_sec
        hi = lo + intlen_sec
        mid = 0.5 * (lo + hi)
        radius = 0.5 * intlen_sec
        mask = (et >= lo - 0.001) & (et <= hi + 0.001)
        idx = np.where(mask)[0]
        if idx.size < degree + 1:
            raise ValueError(f'interval {k} has {idx.size} samples for degree {degree}; increase sampling density (smaller step_days)')
        s = (et[idx] - mid) / radius
        coeffs_axes: list[np.ndarray] = []
        for axis in range(3):
            c = np.polynomial.chebyshev.chebfit(s, pos_km[idx, axis], degree)
            coeffs_axes.append(c)
            recon = np.polynomial.chebyshev.chebval(s, c)
            resid = np.abs(recon - pos_km[idx, axis])
            jmax = int(np.argmax(resid))
            if resid[jmax] > worst_resid:
                worst_resid = float(resid[jmax])
                worst_jd = float(jd[idx[jmax]])
        rec = [mid, radius]
        for c in coeffs_axes:
            rec.extend(c.tolist())
        records.append(rec)
    seg = Type2Segment(target=target, center=center, frame=frame, init_sec=t0, intlen_sec=intlen_sec, rsize=rsize, records=records, segid=segid)
    stats = {'n_records': n_records, 'degree': degree, 'intlen_days': intlen_days, 'worst_insample_resid_km': worst_resid, 'worst_insample_jd_tdb': worst_jd, 'et_start': t0, 'et_end': t_end}
    return (seg, stats)

import hashlib
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/jpl'))
import numpy as np
from cheby_fit import fit_type2_segment
from horizons import _parse_vectors
from jplephem.spk import SPK
from spk_writer import write_type2_spk
from fit_gate import assert_fit_within_floor
from validate import angular_sep_arcsec


def run():
    path = ROOT / 'tests/fixtures/ceres-vectors.csv'
    identity = json.loads(path.with_suffix('.json').read_text())
    assert hashlib.sha256(path.read_bytes()).hexdigest() == identity['subset_sha256']
    data = path.read_text()
    jd, positions, _ = _parse_vectors('$$SOE\n' + data + '\n$$EOE')
    segment, report = fit_type2_segment(jd, positions, target=2000001, center=10, frame=1,
                                      intlen_days=32, degree=12, segid='official Horizons offline test')
    assert report['n_records'] == 4
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'ceres.bsp'
        write_type2_spk(str(path), segment, internal_name='official Horizons offline test')
        spk = SPK.open(path)
        try:
            actual = np.asarray([spk[10, 2000001].compute(epoch) for epoch in jd])
        finally:
            spk.close()
    residual = float(angular_sep_arcsec(actual, positions).max())
    assert_fit_within_floor('Ceres offline slice', residual)
    print(f'Pinned official-input fit and SPK readback passed: {len(jd)} vectors, {residual} arcseconds')


if __name__ == '__main__':
    run()

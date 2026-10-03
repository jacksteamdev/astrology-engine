# Copyright (c) Jack Asher
# SPDX-License-Identifier: MPL-2.0

from __future__ import annotations
import struct
from collections import namedtuple
DBL = 8
RCRD = 1024
RCRD_DBL = RCRD // DBL
ND = 2
NI = 6

Type2Segment = namedtuple('Type2Segment', 'target center frame init_sec intlen_sec rsize records segid', defaults=['offline Chebyshev fit'])


def _pad(b: bytes, n: int, fill: bytes=b' ') -> bytes:
    return b[:n].ljust(n, fill)

def write_type2_spk(path: str, seg: Type2Segment, internal_name: str) -> dict:
    data: list[float] = []
    for rec in seg.records:
        if len(rec) != seg.rsize:
            raise ValueError(f'record has {len(rec)} f64 but rsize is {seg.rsize} — fit/serialize mismatch')
        data.extend(rec)
    n_records = len(seg.records)
    directory = [seg.init_sec, seg.intlen_sec, float(seg.rsize), float(n_records)]
    data.extend(directory)
    n_doubles = len(data)
    seg_bytes = struct.pack(f'<{n_doubles}d', *data)
    start_idx = 3 * RCRD_DBL + 1
    end_idx = start_idx + n_doubles - 1
    final_epoch = seg.init_sec + seg.intlen_sec * n_records
    fr = bytearray(b'\x00' * RCRD)
    fr[0:8] = b'DAF/SPK '
    struct.pack_into('<ii', fr, 8, ND, NI)
    fr[16:76] = _pad(internal_name.encode('ascii'), 60)
    struct.pack_into('<iii', fr, 76, 2, 2, end_idx + 1)
    fr[88:96] = b'LTL-IEEE'
    ftp = b'FTPSTR:\r:\n:\r\n:\r\x00:\x81:\x10\xce:ENDFTP'
    fr[699:699 + len(ftp)] = ftp
    summary_size = ND + (NI + 1) // 2
    sr = bytearray(b'\x00' * RCRD)
    struct.pack_into('<ddd', sr, 0, 0.0, 0.0, 1.0)
    off = 24
    struct.pack_into('<dd', sr, off, seg.init_sec, final_epoch)
    off += ND * DBL
    struct.pack_into('<iiiiii', sr, off, seg.target, seg.center, seg.frame, 2, start_idx, end_idx)
    nr = bytearray(b' ' * RCRD)
    name = _pad(seg.segid.encode('ascii'), summary_size * DBL)
    nr[0:len(name)] = name
    with open(path, 'wb') as f:
        f.write(bytes(fr))
        f.write(bytes(sr))
        f.write(bytes(nr))
        f.write(seg_bytes)
    total = 3 * RCRD + len(seg_bytes)
    return {'path': path, 'target': seg.target, 'center': seg.center, 'frame': seg.frame, 'n_records': n_records, 'rsize': seg.rsize, 'degree': (seg.rsize - 2) // 3 - 1, 'init_sec': seg.init_sec, 'final_sec': final_epoch, 'intlen_sec': seg.intlen_sec, 'start_idx': start_idx, 'end_idx': end_idx, 'bytes': total}

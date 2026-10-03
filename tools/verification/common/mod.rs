// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

pub fn crc(bytes: &[u8]) -> u32 {
    let mut value = u32::MAX;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = if value & 1 == 1 {
                (value >> 1) ^ 0xedb88320
            } else {
                value >> 1
            };
        }
    }
    !value
}

pub fn refresh_table(blob: &mut [u8]) {
    let count = u32::from_le_bytes(blob[12..16].try_into().unwrap()) as usize;
    let checksum = crc(&blob[12..64 + 48 * count]);
    blob[8..12].copy_from_slice(&checksum.to_le_bytes());
}

pub fn polynomial_blob() -> Vec<u8> {
    let pairs: Vec<_> = (0_u16..13)
        .flat_map(|body| {
            (0_u16..if body == 12 { 1 } else { 2 }).map(move |quantity| (body, quantity))
        })
        .collect();
    let table_end = 64 + 48 * pairs.len();
    let mut blob = vec![0; table_end + pairs.len() * 16];
    let length = blob.len() as u64;
    blob[..8].copy_from_slice(b"HDCHEB01");
    blob[12..16].copy_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (at, value) in [
        (16, -50_000_000.0_f64),
        (24, 50_000_000.0),
        (32, -100_000_000.0),
        (40, 100_000_000.0),
    ] {
        blob[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }
    blob[48..56].copy_from_slice(&length.to_le_bytes());
    for (index, (body, quantity)) in pairs.into_iter().enumerate() {
        let at = 64 + 48 * index;
        blob[at..at + 2].copy_from_slice(&body.to_le_bytes());
        blob[at + 2..at + 4].copy_from_slice(&quantity.to_le_bytes());
        blob[at + 4..at + 8].copy_from_slice(&2_u32.to_le_bytes());
        blob[at + 8..at + 16].copy_from_slice(&(-100_000_000.0_f64).to_le_bytes());
        blob[at + 16..at + 24].copy_from_slice(&200_000_000.0_f64.to_le_bytes());
        blob[at + 24..at + 28].copy_from_slice(&1_u32.to_le_bytes());
        let offset = table_end + index * 16;
        blob[at + 32..at + 40].copy_from_slice(&(offset as u64).to_le_bytes());
        let constant = if quantity == 0 {
            47.0 + f64::from(body) * 11.0
        } else {
            -6.0 + f64::from(body)
        };
        let slope = if quantity == 0 { 2.0_f64 } else { 0.0_f64 };
        blob[offset..offset + 8].copy_from_slice(&constant.to_le_bytes());
        blob[offset + 8..offset + 16].copy_from_slice(&slope.to_le_bytes());
    }
    let checksum = crc(&blob[table_end..]);
    blob[56..60].copy_from_slice(&checksum.to_le_bytes());
    refresh_table(&mut blob);
    blob
}

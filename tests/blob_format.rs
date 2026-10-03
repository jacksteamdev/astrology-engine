// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

#![cfg(feature = "generation")]

use astrology_engine::tooling::format::{
    crc32, parse, write_blob, BlobError, Quantity, SeriesBody, WriteSeries, HEADER_LEN,
    SERIES_ENTRY_LEN,
};

fn sample_blob() -> Vec<u8> {
    let lon = WriteSeries {
        body: SeriesBody::Sun,
        quantity: Quantity::Longitude,
        n_coeffs: 4,
        t0_et: -1000.0,
        seg_len_s: 500.0,
        coeffs: (0..12).map(|i| i as f64 * 0.25).collect(), // 3 segments × 4 coeffs
    };
    let dec = WriteSeries {
        body: SeriesBody::Sun,
        quantity: Quantity::Declination,
        n_coeffs: 3,
        t0_et: -1000.0,
        seg_len_s: 750.0,
        coeffs: (0..6).map(|i| -(i as f64)).collect(), // 2 segments × 3 coeffs
    };
    write_blob(-900.0, 400.0, -1000.0, 500.0, 0xC0FFEE, &[lon, dec])
}

#[test]
fn round_trips_header_and_table() {
    let blob = sample_blob();
    let (header, table) = parse(&blob).expect("well-formed blob parses");

    assert_eq!(header.n_series, 2);
    assert_eq!(header.domain_lo_et, -900.0);
    assert_eq!(header.domain_hi_et, 400.0);
    assert_eq!(header.fitted_lo_et, -1000.0);
    assert_eq!(header.fitted_hi_et, 500.0);
    assert_eq!(header.total_len, blob.len() as u64);
    assert_eq!(header.engine_fingerprint, 0xC0FFEE);

    assert_eq!(table.len(), 2);
    assert_eq!(table[0].body, SeriesBody::Sun);
    assert_eq!(table[0].quantity, Quantity::Longitude);
    assert_eq!(table[0].n_coeffs, 4);
    assert_eq!(table[0].n_segments, 3);
    assert_eq!(table[1].quantity, Quantity::Declination);
    assert_eq!(table[1].n_segments, 2);

    // Coefficients land where the table says they do.
    let at = table[0].byte_offset as usize;
    let first = f64::from_le_bytes(blob[at..at + 8].try_into().unwrap());
    assert_eq!(first, 0.0);
    let at1 = table[1].byte_offset as usize;
    let dec_first = f64::from_le_bytes(blob[at1..at1 + 8].try_into().unwrap());
    assert_eq!(dec_first, 0.0);
    assert_eq!(
        table[1].byte_offset,
        table[0].byte_offset + table[0].coeff_bytes()
    );
}

#[test]
fn full_crc_covers_the_coefficient_region() {
    let blob = sample_blob();
    let (header, _) = parse(&blob).expect("parse");
    let table_end = HEADER_LEN + 2 * SERIES_ENTRY_LEN;
    assert_eq!(header.full_crc32, crc32(&blob[table_end..]));
}

#[test]
fn rejects_a_truncated_blob() {
    let blob = sample_blob();
    let cut = &blob[..blob.len() - 9];
    match parse(cut) {
        Err(BlobError::LengthMismatch { declared, actual }) => {
            assert_eq!(declared, blob.len() as u64);
            assert_eq!(actual, cut.len());
        }
        other => panic!("truncated blob should fail on length, got {other:?}"),
    }
    assert!(matches!(
        parse(&blob[..40]),
        Err(BlobError::TooShort { .. })
    ));
}

#[test]
fn rejects_a_foreign_magic() {
    let mut blob = sample_blob();
    blob[0..8].copy_from_slice(b"DAF/SPK "); // the old world knocking
    assert!(matches!(parse(&blob), Err(BlobError::BadMagic)));
}

#[test]
fn rejects_a_corrupted_series_table() {
    let mut blob = sample_blob();
    // Flip one byte inside the table region — the table crc must notice.
    blob[HEADER_LEN + 5] ^= 0xFF;
    assert!(matches!(parse(&blob), Err(BlobError::BadTableCrc { .. })));
}

#[test]
fn rejects_out_of_bounds_coefficients_even_with_a_valid_crc() {
    let mut blob = sample_blob();
    // Point series 0 past the end of the blob, then re-sign the table so only the
    // bounds check can object.
    let at = HEADER_LEN + 32;
    let past_end = blob.len() as u64;
    blob[at..at + 8].copy_from_slice(&past_end.to_le_bytes());
    let table_end = HEADER_LEN + 2 * SERIES_ENTRY_LEN;
    let fixed_crc = crc32(&blob[12..table_end]);
    blob[8..12].copy_from_slice(&fixed_crc.to_le_bytes());
    match parse(&blob) {
        Err(BlobError::BadSeries(msg)) => assert!(msg.contains("outside the blob"), "{msg}"),
        other => panic!("expected BadSeries, got {other:?}"),
    }
}

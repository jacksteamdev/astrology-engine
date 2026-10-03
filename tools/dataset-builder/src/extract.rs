//! Lossless date-range extraction. No kernel acquisition or refitting.
use crate::cheb::format::{self, Header, SeriesMeta, WriteSeries};
use hifitime::{Epoch, TimeScale};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn utc(et: f64) -> String {
    Epoch::from_et_seconds(et)
        .to_gregorian_str(TimeScale::UTC)
        .replace(" UTC", "Z")
}
pub fn parse_utc(value: &str) -> Result<Epoch, String> {
    if !value.ends_with('Z') && !value.ends_with(" UTC") {
        return Err("Use an explicit UTC timestamp ending in Z or UTC".into());
    }
    value.parse::<Epoch>().map_err(|e| e.to_string())
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn checked(bytes: &[u8]) -> Result<(Header, Vec<SeriesMeta>), String> {
    let (h, table) = format::parse(bytes).map_err(|e| e.to_string())?;
    let table_end = format::HEADER_LEN + table.len() * format::SERIES_ENTRY_LEN;
    if format::crc32(&bytes[table_end..]) != h.full_crc32 {
        return Err("Coefficient checksum mismatch".into());
    }
    if h.fitted_lo_et > h.domain_lo_et || h.fitted_hi_et < h.domain_hi_et {
        return Err("Public domain exceeds fitted support".into());
    }
    for (i, s) in table.iter().enumerate() {
        if !s.end_et().is_finite() || s.t0_et > h.fitted_lo_et || s.end_et() < h.fitted_hi_et {
            return Err(format!("Series {i} does not cover the fitted span"));
        }
        if table[..i]
            .iter()
            .any(|p| p.body == s.body && p.quantity == s.quantity)
        {
            return Err(format!("Duplicate series {i}"));
        }
        let start = s.byte_offset as usize;
        let end = start + s.coeff_bytes() as usize;
        if bytes[start..end]
            .chunks_exact(8)
            .any(|c| !f64::from_le_bytes(c.try_into().unwrap()).is_finite())
        {
            return Err(format!("Non-finite coefficient in series {i}"));
        }
    }
    Ok((h, table))
}

pub fn inspect(bytes: &[u8]) -> Result<Value, String> {
    let (h, _) = checked(bytes)?;
    Ok(json!({
        "format": "HDCHEB01", "bytes": bytes.len(), "sha256": sha256(bytes),
        "engineFingerprint": format!("{:08x}", h.engine_fingerprint), "seriesCount": h.n_series,
        "domainEt": {"lo": h.domain_lo_et, "hi": h.domain_hi_et},
        "domainUtc": {"lo": utc(h.domain_lo_et), "hi": utc(h.domain_hi_et)},
        "fittedEt": {"lo": h.fitted_lo_et, "hi": h.fitted_hi_et},
        "fittedUtc": {"lo": utc(h.fitted_lo_et), "hi": utc(h.fitted_hi_et)}
    }))
}

pub fn extract(bytes: &[u8], lo: f64, hi: f64) -> Result<(Vec<u8>, Value), String> {
    let (h, table) = checked(bytes)?;
    if !lo.is_finite() || !hi.is_finite() || lo >= hi || lo < h.domain_lo_et || hi > h.domain_hi_et
    {
        return Err("Extraction bounds must be ordered and inside the source public domain".into());
    }
    let support_lo = lo - (h.domain_lo_et - h.fitted_lo_et).max(43_200.0);
    let support_hi = hi + (h.fitted_hi_et - h.domain_hi_et).max(43_200.0);
    let mut series = Vec::with_capacity(table.len());
    for s in table {
        if support_lo < s.t0_et || support_hi > s.end_et() {
            return Err("Insufficient fitted support for extraction guards".into());
        }
        let first = ((support_lo - s.t0_et) / s.seg_len_s).floor() as usize;
        // Include the right-hand block at an exact interior boundary, matching evaluation.
        let last = (((support_hi - s.t0_et) / s.seg_len_s).floor() as usize)
            .min(s.n_segments as usize - 1);
        let stride = s.n_coeffs as usize * 8;
        let begin = s.byte_offset as usize + first * stride;
        let end = s.byte_offset as usize + (last + 1) * stride;
        series.push(WriteSeries {
            body: s.body,
            quantity: s.quantity,
            n_coeffs: s.n_coeffs,
            t0_et: s.t0_et + first as f64 * s.seg_len_s,
            seg_len_s: s.seg_len_s,
            coeffs: bytes[begin..end]
                .chunks_exact(8)
                .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                .collect(),
        });
    }
    // Describe guaranteed support; individual blocks can extend farther.
    let output = format::write_blob(
        lo,
        hi,
        support_lo,
        support_hi,
        h.engine_fingerprint,
        &series,
    );
    let mut manifest = inspect(&output)?;
    manifest["sourceSha256"] = json!(sha256(bytes));
    Ok((output, manifest))
}

pub fn command(args: &[String]) -> Result<(), String> {
    use std::{
        fs::{self, OpenOptions},
        io::Write,
        path::Path,
    };
    let mode = args.first().ok_or("Expected extract or inspect")?;
    let allowed: &[&str] = if mode == "inspect" {
        &["--input"]
    } else {
        &["--input", "--start-utc", "--end-utc", "--out", "--manifest"]
    };
    let mut flags = std::collections::BTreeMap::new();
    for pair in args[1..].chunks(2) {
        if pair.len() != 2
            || !allowed.contains(&pair[0].as_str())
            || flags.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("Unknown, duplicate or missing command argument".into());
        }
    }
    let get = |name: &str| {
        flags
            .get(name)
            .copied()
            .ok_or_else(|| format!("Required {name}"))
    };
    let input = fs::read(get("--input")?).map_err(|e| e.to_string())?;
    if mode == "inspect" {
        println!("{}", inspect(&input)?);
        return Ok(());
    }
    let lo = parse_utc(get("--start-utc")?)?.to_et_seconds();
    let hi = parse_utc(get("--end-utc")?)?.to_et_seconds();
    let (bytes, manifest) = extract(&input, lo, hi)?;
    let out = Path::new(get("--out")?);
    let meta = Path::new(get("--manifest")?);
    for path in [out, meta] {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    // Never overwrite source data or an earlier extraction.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .map_err(|e| e.to_string())?;
    let mut metadata = match OpenOptions::new().write(true).create_new(true).open(meta) {
        Ok(file) => file,
        Err(e) => {
            let _ = fs::remove_file(out);
            return Err(e.to_string());
        }
    };
    let result = file
        .write_all(&bytes)
        .and_then(|()| metadata.write_all(format!("{manifest}\n").as_bytes()));
    if let Err(e) = result {
        let _ = fs::remove_file(out);
        let _ = fs::remove_file(meta);
        return Err(e.to_string());
    }
    println!("{manifest}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use format::{Quantity, SeriesBody};
    fn fixture() -> Vec<u8> {
        let series = [8.0, 16.0, 32.0]
            .iter()
            .enumerate()
            .map(|(i, days)| WriteSeries {
                body: SeriesBody::ALL[i],
                quantity: Quantity::Longitude,
                n_coeffs: 2,
                t0_et: -640.0 * 86400.0,
                seg_len_s: days * 86400.0,
                coeffs: (0..(1280.0 / days) as usize * 2)
                    .map(|n| n as f64 / 17.0)
                    .collect(),
            })
            .collect::<Vec<_>>();
        format::write_blob(
            -635.0 * 86400.0,
            639.0 * 86400.0,
            -640.0 * 86400.0,
            640.0 * 86400.0,
            123,
            &series,
        )
    }
    #[test]
    fn copies_blocks_and_preserves_guarded_endpoints() {
        let source = fixture();
        let (original, original_table) = checked(&source).unwrap();
        for (lo, hi) in [
            (-32.0 * 86400.0, 32.0 * 86400.0),
            (original.domain_lo_et, original.domain_hi_et),
            (-86400.125, 86400.125),
        ] {
            let (bytes, manifest) = extract(&source, lo, hi).unwrap();
            let (h, table) = checked(&bytes).unwrap();
            assert_eq!(h.domain_lo_et, lo);
            assert_eq!(h.domain_hi_et, hi);
            assert_eq!(h.engine_fingerprint, 123);
            assert_eq!(lo - h.fitted_lo_et, 5.0 * 86400.0);
            assert_eq!(h.fitted_hi_et - hi, 86400.0);
            assert_eq!(manifest["sourceSha256"], sha256(&source));
            for (s, old) in table.iter().zip(&original_table) {
                let offset =
                    ((s.t0_et - old.t0_et) / old.seg_len_s) as usize * old.n_coeffs as usize * 8;
                let start = old.byte_offset as usize + offset;
                assert_eq!(
                    &bytes[s.byte_offset as usize..(s.byte_offset + s.coeff_bytes()) as usize],
                    &source[start..start + s.coeff_bytes() as usize]
                );
            }
            let full = astrology_engine::Ephemeris::parse(source.clone()).unwrap();
            let small = astrology_engine::Ephemeris::parse(bytes).unwrap();
            for t in [lo - 43200.0, lo, 0.0, hi, hi + 43200.0] {
                for b in [SeriesBody::Sun, SeriesBody::Moon, SeriesBody::Mercury] {
                    assert_eq!(
                        full.longitude(b, t).unwrap().to_bits(),
                        small.longitude(b, t).unwrap().to_bits()
                    );
                }
            }
        }
    }
    #[test]
    fn exact_support_boundary_and_narrow_source_edges() {
        let source = fixture();
        let (h, _) = checked(&source).unwrap();
        for (lo, hi) in [
            (-86400.0, 31.0 * 86400.0),
            (h.domain_lo_et, h.domain_lo_et + 86400.0),
            (h.domain_hi_et - 86400.0, h.domain_hi_et),
        ] {
            let (bytes, _) = extract(&source, lo, hi).unwrap();
            let (part, _) = checked(&bytes).unwrap();
            let full = astrology_engine::Ephemeris::parse(source.clone()).unwrap();
            let small = astrology_engine::Ephemeris::parse(bytes).unwrap();
            for t in [
                part.fitted_lo_et,
                lo,
                hi,
                part.fitted_hi_et - 0.001,
                part.fitted_hi_et,
            ] {
                for b in [SeriesBody::Sun, SeriesBody::Moon, SeriesBody::Mercury] {
                    assert_eq!(
                        full.longitude(b, t).unwrap().to_bits(),
                        small.longitude(b, t).unwrap().to_bits()
                    );
                }
            }
        }
    }
    #[test]
    fn rejects_malformed_series_and_short_speed_support() {
        let patch_crc = |bytes: &mut Vec<u8>| {
            let table_end = 64 + 3 * 48;
            let full = format::crc32(&bytes[table_end..]);
            bytes[56..60].copy_from_slice(&full.to_le_bytes());
            let table = format::crc32(&bytes[12..table_end]);
            bytes[8..12].copy_from_slice(&table.to_le_bytes());
        };
        let mut bytes = fixture();
        bytes[112..114].copy_from_slice(&0_u16.to_le_bytes());
        patch_crc(&mut bytes);
        assert!(inspect(&bytes).unwrap_err().contains("Duplicate"));
        let mut bytes = fixture();
        let len = bytes.len();
        bytes[len - 8..].copy_from_slice(&f64::NAN.to_le_bytes());
        patch_crc(&mut bytes);
        assert!(inspect(&bytes).unwrap_err().contains("Non-finite"));
        let mut bytes = fixture();
        bytes[96..104].copy_from_slice(&u64::MAX.to_le_bytes());
        patch_crc(&mut bytes);
        assert!(inspect(&bytes).is_err());
        let mut bytes = fixture();
        bytes[16..24].copy_from_slice(&(-640.0_f64 * 86400.0).to_le_bytes());
        patch_crc(&mut bytes);
        assert!(extract(&bytes, -640.0 * 86400.0, 0.0)
            .unwrap_err()
            .contains("Insufficient"));
    }
    #[test]
    fn rejects_corruption_bad_ranges_and_missing_guards() {
        let mut source = fixture();
        for (lo, hi) in [(1.0, 0.0), (0.0, f64::NAN), (-1e10, 1.0)] {
            assert!(extract(&source, lo, hi).is_err());
        }
        *source.last_mut().unwrap() ^= 1;
        assert!(inspect(&source).unwrap_err().contains("checksum"));
        let mut source = fixture();
        source[32..40].copy_from_slice(&0.0_f64.to_le_bytes());
        let crc = format::crc32(&source[12..64 + 3 * 48]);
        source[8..12].copy_from_slice(&crc.to_le_bytes());
        assert!(inspect(&source).is_err());
        assert!(parse_utc("2000-01-01T00:00:00").is_err());
        assert!(parse_utc("2000-02-29T00:00:00Z").is_ok());
    }
}

// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use super::fit::clenshaw;
use super::format::{self, BlobError, Header, Quantity, SeriesBody, SeriesMeta};
use crate::astro::find_moment::{normalize360, signed_delta};
use crate::astro::{true_obliquity, AstroTime};
use crate::types::{Body, ChartBody};
use hifitime::{Duration, Epoch};
const SPEED_HALF_STEP_DAYS: f64 = 0.5;
const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvalError {
    OutOfDomain {
        et: f64,
    },
    MissingSeries {
        body: SeriesBody,
        quantity: Quantity,
    },
}
impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::OutOfDomain { et } => {
                write!(f, "epoch {et} s (ET) is outside the fitted ephemeris span")
            }
            EvalError::MissingSeries { body, quantity } => {
                write!(f, "blob has no series for {body:?}/{quantity:?}")
            }
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ChebPlace {
    pub longitude: f64,
    pub speed: f64,
    pub declination: f64,
}
pub struct Ephemeris {
    bytes: Vec<u8>,
    header: Header,
    table: Vec<SeriesMeta>,
}
impl Ephemeris {
    pub fn parse(bytes: Vec<u8>) -> Result<Ephemeris, BlobError> {
        let (header, table) = format::parse(&bytes)?;
        Ok(Ephemeris {
            bytes,
            header,
            table,
        })
    }
    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn domain_et(&self) -> (f64, f64) {
        (self.header.domain_lo_et, self.header.domain_hi_et)
    }
    fn series(&self, body: SeriesBody, quantity: Quantity) -> Option<&SeriesMeta> {
        self.table
            .iter()
            .find(|s| s.body == body && s.quantity == quantity)
    }
    fn value_at(&self, body: SeriesBody, quantity: Quantity, et: f64) -> Result<f64, EvalError> {
        let meta = self
            .series(body, quantity)
            .ok_or(EvalError::MissingSeries { body, quantity })?;
        let rel = (et - meta.t0_et) / meta.seg_len_s;
        let mut idx = rel.floor();
        if idx == meta.n_segments as f64 && et <= meta.end_et() {
            idx -= 1.0;
        }
        if idx < 0.0 || idx >= meta.n_segments as f64 {
            return Err(EvalError::OutOfDomain { et });
        }
        let i = idx as usize;
        let seg_lo = meta.t0_et + i as f64 * meta.seg_len_s;
        let x = 2.0 * (et - seg_lo) / meta.seg_len_s - 1.0;
        let n = meta.n_coeffs as usize;
        let at = meta.byte_offset as usize + i * n * 8;
        let mut coeffs = [0.0_f64; 64];
        for (k, c) in coeffs.iter_mut().take(n).enumerate() {
            *c = format::read_f64(&self.bytes, at + k * 8);
        }
        Ok(clenshaw(&coeffs[..n], x))
    }
    pub fn longitude(&self, body: SeriesBody, et: f64) -> Result<f64, EvalError> {
        Ok(normalize360(self.value_at(
            body,
            Quantity::Longitude,
            et,
        )?))
    }
    pub fn declination(&self, body: SeriesBody, et: f64) -> Result<f64, EvalError> {
        self.value_at(body, Quantity::Declination, et)
    }
    pub fn place(&self, body: SeriesBody, tdb: Epoch) -> Result<ChebPlace, EvalError> {
        let et = tdb.to_et_seconds();
        let longitude = self.longitude(body, et)?;
        let declination = self.declination(body, et)?;
        let speed = self.fd_speed(body, tdb)?;
        Ok(ChebPlace {
            longitude,
            speed,
            declination,
        })
    }
    fn fd_speed(&self, body: SeriesBody, tdb: Epoch) -> Result<f64, EvalError> {
        let after = self.longitude(
            body,
            (tdb + Duration::from_days(SPEED_HALF_STEP_DAYS)).to_et_seconds(),
        )?;
        let before = self.longitude(
            body,
            (tdb - Duration::from_days(SPEED_HALF_STEP_DAYS)).to_et_seconds(),
        )?;
        Ok(signed_delta(after, before) / (2.0 * SPEED_HALF_STEP_DAYS))
    }
    pub fn node_pair(&self, tdb: Epoch) -> Result<(ChartBody, ChartBody), EvalError> {
        let et = tdb.to_et_seconds();
        let north_longitude = self.longitude(SeriesBody::NodeOmega, et)?;
        let south_longitude = normalize360(north_longitude + 180.0);
        let speed = self.fd_speed(SeriesBody::NodeOmega, tdb)?;
        let eps = true_obliquity(AstroTime::from_tdb(tdb)) * DEG2RAD;
        let declination = (eps.sin() * (north_longitude * DEG2RAD).sin()).asin() * RAD2DEG;
        Ok((
            ChartBody {
                name: Body::NorthNode.wire_name().to_string(),
                longitude: north_longitude,
                speed,
                declination,
            },
            ChartBody {
                name: Body::SouthNode.wire_name().to_string(),
                longitude: south_longitude,
                speed,
                declination,
            },
        ))
    }
}

use crate::types::Body;
pub const MAGIC: [u8; 8] = *b"HDCHEB01";
pub const HEADER_LEN: usize = 64;
pub const SERIES_ENTRY_LEN: usize = 48;
pub const MAX_SERIES: u32 = 256;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesBody {
    Sun = 0,
    Moon = 1,
    Mercury = 2,
    Venus = 3,
    Mars = 4,
    Jupiter = 5,
    Saturn = 6,
    Uranus = 7,
    Neptune = 8,
    Pluto = 9,
    Chiron = 10,
    Ceres = 11,
    NodeOmega = 12,
}
impl SeriesBody {
    pub const ALL: [SeriesBody; 13] = [
        SeriesBody::Sun,
        SeriesBody::Moon,
        SeriesBody::Mercury,
        SeriesBody::Venus,
        SeriesBody::Mars,
        SeriesBody::Jupiter,
        SeriesBody::Saturn,
        SeriesBody::Uranus,
        SeriesBody::Neptune,
        SeriesBody::Pluto,
        SeriesBody::Chiron,
        SeriesBody::Ceres,
        SeriesBody::NodeOmega,
    ];
    pub fn from_u16(v: u16) -> Option<SeriesBody> {
        SeriesBody::ALL.into_iter().find(|b| *b as u16 == v)
    }
    pub fn from_body(body: Body) -> Option<SeriesBody> {
        match body {
            Body::Sun => Some(SeriesBody::Sun),
            Body::Moon => Some(SeriesBody::Moon),
            Body::Mercury => Some(SeriesBody::Mercury),
            Body::Venus => Some(SeriesBody::Venus),
            Body::Mars => Some(SeriesBody::Mars),
            Body::Jupiter => Some(SeriesBody::Jupiter),
            Body::Saturn => Some(SeriesBody::Saturn),
            Body::Uranus => Some(SeriesBody::Uranus),
            Body::Neptune => Some(SeriesBody::Neptune),
            Body::Pluto => Some(SeriesBody::Pluto),
            Body::Chiron => Some(SeriesBody::Chiron),
            Body::Ceres => Some(SeriesBody::Ceres),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quantity {
    Longitude = 0,
    Declination = 1,
}
impl Quantity {
    pub fn from_u16(v: u16) -> Option<Quantity> {
        match v {
            0 => Some(Quantity::Longitude),
            1 => Some(Quantity::Declination),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub n_series: u32,
    pub domain_lo_et: f64,
    pub domain_hi_et: f64,
    pub fitted_lo_et: f64,
    pub fitted_hi_et: f64,
    pub total_len: u64,
    pub full_crc32: u32,
    pub engine_fingerprint: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct SeriesMeta {
    pub body: SeriesBody,
    pub quantity: Quantity,
    pub n_coeffs: u32,
    pub t0_et: f64,
    pub seg_len_s: f64,
    pub n_segments: u32,
    pub byte_offset: u64,
}
impl SeriesMeta {
    pub fn coeff_bytes(&self) -> u64 {
        self.n_segments as u64 * self.n_coeffs as u64 * 8
    }
    pub fn end_et(&self) -> f64 {
        self.t0_et + self.n_segments as f64 * self.seg_len_s
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum BlobError {
    TooShort { len: usize },
    BadMagic,
    LengthMismatch { declared: u64, actual: usize },
    BadTableCrc { declared: u32, computed: u32 },
    BadSeries(String),
}
impl std::fmt::Display for BlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::TooShort { len } => {
                write!(f, "blob too short for an HDCHEB01 header: {len} bytes")
            }
            BlobError::BadMagic => write!(f, "not an HDCHEB01 blob (magic mismatch)"),
            BlobError::LengthMismatch { declared, actual } => write!(
                f,
                "blob length mismatch: header says {declared} bytes, received {actual}"
            ),
            BlobError::BadTableCrc { declared, computed } => write!(
                f,
                "series-table crc mismatch: header {declared:#010x}, computed {computed:#010x}"
            ),
            BlobError::BadSeries(msg) => write!(f, "bad series entry: {msg}"),
        }
    }
}
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}
fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
fn read_u64(bytes: &[u8], at: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(buf)
}
pub(crate) fn read_f64(bytes: &[u8], at: usize) -> f64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[at..at + 8]);
    f64::from_le_bytes(buf)
}
pub fn parse(bytes: &[u8]) -> Result<(Header, Vec<SeriesMeta>), BlobError> {
    if bytes.len() < HEADER_LEN {
        return Err(BlobError::TooShort { len: bytes.len() });
    }
    if bytes[0..8] != MAGIC {
        return Err(BlobError::BadMagic);
    }
    let declared_table_crc = read_u32(bytes, 8);
    let n_series = read_u32(bytes, 12);
    if n_series == 0 || n_series > MAX_SERIES {
        return Err(BlobError::BadSeries(format!(
            "implausible series count {n_series}"
        )));
    }
    let header = Header {
        n_series,
        domain_lo_et: read_f64(bytes, 16),
        domain_hi_et: read_f64(bytes, 24),
        fitted_lo_et: read_f64(bytes, 32),
        fitted_hi_et: read_f64(bytes, 40),
        total_len: read_u64(bytes, 48),
        full_crc32: read_u32(bytes, 56),
        engine_fingerprint: read_u32(bytes, 60),
    };
    if header.total_len != bytes.len() as u64 {
        return Err(BlobError::LengthMismatch {
            declared: header.total_len,
            actual: bytes.len(),
        });
    }
    let table_end = HEADER_LEN + SERIES_ENTRY_LEN * n_series as usize;
    if bytes.len() < table_end {
        return Err(BlobError::TooShort { len: bytes.len() });
    }
    let computed_crc = crc32(&bytes[12..table_end]);
    if computed_crc != declared_table_crc {
        return Err(BlobError::BadTableCrc {
            declared: declared_table_crc,
            computed: computed_crc,
        });
    }
    if !(header.domain_lo_et.is_finite()
        && header.domain_hi_et.is_finite()
        && header.fitted_lo_et.is_finite()
        && header.fitted_hi_et.is_finite()
        && header.domain_lo_et < header.domain_hi_et)
    {
        return Err(BlobError::BadSeries(
            "non-finite or inverted domain".to_string(),
        ));
    }
    let mut table = Vec::with_capacity(n_series as usize);
    for i in 0..n_series as usize {
        let at = HEADER_LEN + SERIES_ENTRY_LEN * i;
        let body = SeriesBody::from_u16(read_u16(bytes, at))
            .ok_or_else(|| BlobError::BadSeries(format!("series {i}: unknown body id")))?;
        let quantity = Quantity::from_u16(read_u16(bytes, at + 2))
            .ok_or_else(|| BlobError::BadSeries(format!("series {i}: unknown quantity")))?;
        let meta = SeriesMeta {
            body,
            quantity,
            n_coeffs: read_u32(bytes, at + 4),
            t0_et: read_f64(bytes, at + 8),
            seg_len_s: read_f64(bytes, at + 16),
            n_segments: read_u32(bytes, at + 24),
            byte_offset: read_u64(bytes, at + 32),
        };
        if meta.n_coeffs == 0 || meta.n_coeffs > 64 {
            return Err(BlobError::BadSeries(format!(
                "series {i}: implausible coefficient count {}",
                meta.n_coeffs
            )));
        }
        if meta.n_segments == 0 || !meta.seg_len_s.is_finite() || meta.seg_len_s <= 0.0 {
            return Err(BlobError::BadSeries(format!(
                "series {i}: bad segmentation"
            )));
        }
        if !meta.t0_et.is_finite() {
            return Err(BlobError::BadSeries(format!("series {i}: non-finite t0")));
        }
        let end = meta
            .byte_offset
            .checked_add(meta.coeff_bytes())
            .ok_or_else(|| BlobError::BadSeries(format!("series {i}: offset overflow")))?;
        if meta.byte_offset < table_end as u64 || end > bytes.len() as u64 {
            return Err(BlobError::BadSeries(format!(
                "series {i}: coefficients [{}..{end}) fall outside the blob",
                meta.byte_offset
            )));
        }
        table.push(meta);
    }
    Ok((header, table))
}
#[cfg(feature = "generation")]
pub struct WriteSeries {
    pub body: SeriesBody,
    pub quantity: Quantity,
    pub n_coeffs: u32,
    pub t0_et: f64,
    pub seg_len_s: f64,
    pub coeffs: Vec<f64>,
}
#[cfg(feature = "generation")]
pub fn write_blob(
    domain_lo_et: f64,
    domain_hi_et: f64,
    fitted_lo_et: f64,
    fitted_hi_et: f64,
    engine_fingerprint: u32,
    series: &[WriteSeries],
) -> Vec<u8> {
    let table_end = HEADER_LEN + SERIES_ENTRY_LEN * series.len();
    let coeff_bytes: usize = series.iter().map(|s| s.coeffs.len() * 8).sum();
    let total_len = table_end + coeff_bytes;
    let mut out = vec![0u8; total_len];
    out[0..8].copy_from_slice(&MAGIC);
    out[12..16].copy_from_slice(&(series.len() as u32).to_le_bytes());
    out[16..24].copy_from_slice(&domain_lo_et.to_le_bytes());
    out[24..32].copy_from_slice(&domain_hi_et.to_le_bytes());
    out[32..40].copy_from_slice(&fitted_lo_et.to_le_bytes());
    out[40..48].copy_from_slice(&fitted_hi_et.to_le_bytes());
    out[48..56].copy_from_slice(&(total_len as u64).to_le_bytes());
    out[60..64].copy_from_slice(&engine_fingerprint.to_le_bytes());
    let mut offset = table_end;
    for (i, s) in series.iter().enumerate() {
        assert!(
            s.n_coeffs > 0 && s.coeffs.len() % s.n_coeffs as usize == 0,
            "series {i}: {} coefficients is not a whole number of degree-{} segments",
            s.coeffs.len(),
            s.n_coeffs.saturating_sub(1),
        );
        let n_segments = (s.coeffs.len() / s.n_coeffs as usize) as u32;
        let at = HEADER_LEN + SERIES_ENTRY_LEN * i;
        out[at..at + 2].copy_from_slice(&(s.body as u16).to_le_bytes());
        out[at + 2..at + 4].copy_from_slice(&(s.quantity as u16).to_le_bytes());
        out[at + 4..at + 8].copy_from_slice(&s.n_coeffs.to_le_bytes());
        out[at + 8..at + 16].copy_from_slice(&s.t0_et.to_le_bytes());
        out[at + 16..at + 24].copy_from_slice(&s.seg_len_s.to_le_bytes());
        out[at + 24..at + 28].copy_from_slice(&n_segments.to_le_bytes());
        out[at + 32..at + 40].copy_from_slice(&(offset as u64).to_le_bytes());
        for (j, c) in s.coeffs.iter().enumerate() {
            out[offset + j * 8..offset + j * 8 + 8].copy_from_slice(&c.to_le_bytes());
        }
        offset += s.coeffs.len() * 8;
    }
    let full_crc = crc32(&out[table_end..]);
    out[56..60].copy_from_slice(&full_crc.to_le_bytes());
    let table_crc = crc32(&out[12..table_end]);
    out[8..12].copy_from_slice(&table_crc.to_le_bytes());
    out
}

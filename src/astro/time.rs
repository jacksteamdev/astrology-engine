use hifitime::Epoch;
pub const DAYS_PER_CENTURY: f64 = 36525.0;
const DAYS_PER_TROPICAL_YEAR: f64 = 365.24217;
#[derive(Debug, Clone, Copy)]
pub struct AstroTime {
    pub ut: f64,
    pub tt: f64,
}
impl AstroTime {
    pub fn from_tdb(tdb: Epoch) -> Self {
        const J2000_JD: f64 = 2_451_545.0;
        let ut = tdb.to_jde_utc_days() - J2000_JD;
        AstroTime {
            ut,
            tt: ut + delta_t_seconds(ut) / 86400.0,
        }
    }
}
pub fn delta_t_seconds(ut: f64) -> f64 {
    let y = 2000.0 + ((ut - 14.0) / DAYS_PER_TROPICAL_YEAR);
    if y < -500.0 {
        let u = (y - 1820.0) / 100.0;
        return -20.0 + 32.0 * u * u;
    }
    if y < 500.0 {
        let u = y / 100.0;
        return poly(
            u,
            &[
                10583.6,
                -1014.41,
                33.78311,
                -5.952053,
                -0.1798452,
                0.022174192,
                0.0090316521,
            ],
        );
    }
    if y < 1600.0 {
        let u = (y - 1000.0) / 100.0;
        return poly(
            u,
            &[
                1574.2,
                -556.01,
                71.23472,
                0.319781,
                -0.8503463,
                -0.005050998,
                0.0083572073,
            ],
        );
    }
    if y < 1700.0 {
        let u = y - 1600.0;
        return 120.0 - 0.9808 * u - 0.01532 * u * u + u * u * u / 7129.0;
    }
    if y < 1800.0 {
        let u = y - 1700.0;
        return poly(
            u,
            &[8.83, 0.1603, -0.0059285, 0.00013336, -1.0 / 1_174_000.0],
        );
    }
    if y < 1860.0 {
        let u = y - 1800.0;
        return poly(
            u,
            &[
                13.72,
                -0.332447,
                0.0068612,
                0.0041116,
                -0.00037436,
                0.0000121272,
                -0.0000001699,
                0.000000000875,
            ],
        );
    }
    if y < 1900.0 {
        let u = y - 1860.0;
        return poly(
            u,
            &[
                7.62,
                0.5737,
                -0.251754,
                0.01680668,
                -0.0004473624,
                1.0 / 233174.0,
            ],
        );
    }
    if y < 1920.0 {
        let u = y - 1900.0;
        return poly(u, &[-2.79, 1.494119, -0.0598939, 0.0061966, -0.000197]);
    }
    if y < 1941.0 {
        let u = y - 1920.0;
        return poly(u, &[21.20, 0.84493, -0.076100, 0.0020936]);
    }
    if y < 1961.0 {
        let u = y - 1950.0;
        return 29.07 + 0.407 * u - u * u / 233.0 + u * u * u / 2547.0;
    }
    if y < 1986.0 {
        let u = y - 1975.0;
        return 45.45 + 1.067 * u - u * u / 260.0 - u * u * u / 718.0;
    }
    if y < 2005.0 {
        let u = y - 2000.0;
        return poly(
            u,
            &[
                63.86,
                0.3345,
                -0.060374,
                0.0017275,
                0.000651814,
                0.00002373599,
            ],
        );
    }
    if y < 2050.0 {
        let u = y - 2000.0;
        return 62.92 + 0.32217 * u + 0.005589 * u * u;
    }
    if y < 2150.0 {
        let u = (y - 1820.0) / 100.0;
        return -20.0 + 32.0 * u * u - 0.5628 * (2150.0 - y);
    }
    let u = (y - 1820.0) / 100.0;
    -20.0 + 32.0 * u * u
}
fn poly(u: f64, c: &[f64]) -> f64 {
    c.iter().rev().fold(0.0, |acc, &ci| acc * u + ci)
}

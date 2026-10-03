use super::eval::{ChebPlace, Ephemeris, EvalError};
use super::format::SeriesBody;
use crate::types::{Body, ChartBody};
use hifitime::Epoch;
const SERIES_ORDER: [(Body, SeriesBody); 12] = [
    (Body::Sun, SeriesBody::Sun),
    (Body::Moon, SeriesBody::Moon),
    (Body::Mercury, SeriesBody::Mercury),
    (Body::Venus, SeriesBody::Venus),
    (Body::Mars, SeriesBody::Mars),
    (Body::Jupiter, SeriesBody::Jupiter),
    (Body::Saturn, SeriesBody::Saturn),
    (Body::Uranus, SeriesBody::Uranus),
    (Body::Neptune, SeriesBody::Neptune),
    (Body::Pluto, SeriesBody::Pluto),
    (Body::Chiron, SeriesBody::Chiron),
    (Body::Ceres, SeriesBody::Ceres),
];
fn chart_body(body: Body, place: &ChebPlace) -> ChartBody {
    ChartBody {
        name: body.wire_name().to_string(),
        longitude: place.longitude,
        speed: place.speed,
        declination: place.declination,
    }
}
pub fn earth_from_sun(sun: &ChebPlace) -> ChartBody {
    ChartBody {
        name: Body::Earth.wire_name().to_string(),
        longitude: (sun.longitude + 180.0).rem_euclid(360.0),
        speed: sun.speed,
        declination: sun.declination,
    }
}
pub fn chart_bodies(eph: &Ephemeris, tdb: Epoch) -> Result<Vec<ChartBody>, EvalError> {
    let mut bodies: Vec<ChartBody> = Vec::with_capacity(15);
    for (body, series) in SERIES_ORDER {
        let place = eph.place(series, tdb)?;
        if body == Body::Sun {
            bodies.push(chart_body(body, &place));
            bodies.push(earth_from_sun(&place));
            continue;
        }
        bodies.push(chart_body(body, &place));
        if body == Body::Moon {
            let (north, south) = eph.node_pair(tdb)?;
            bodies.push(north);
            bodies.push(south);
        }
    }
    debug_assert_eq!(bodies.len(), 15);
    Ok(bodies)
}

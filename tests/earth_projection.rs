use astrology_engine::{earth_from_sun, ChartBody, ChebPlace};

#[test]
fn earth_projection_preserves_the_incumbent_body_contract() {
    let sun = ChebPlace {
        longitude: 190.0,
        speed: -0.25,
        declination: -17.5,
    };
    assert_eq!(
        earth_from_sun(&sun),
        ChartBody {
            name: "EARTH".into(),
            longitude: 10.0,
            speed: -0.25,
            declination: -17.5,
        }
    );
}

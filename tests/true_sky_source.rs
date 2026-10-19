//! Preserve the approved source capture moved from the website.
use astrology_engine::{
    assign_sign, true_sky_offset, whole_sign_cusps, zodiac_sectors, Epoch, Ophiuchus,
    SignDivisions, ZodiacConfiguration, ZodiacReference,
};
use serde_json::Value;
use std::str::FromStr;
#[test]
fn approved_source_constellation_and_projection_contract() {
    let source: Value =
        serde_json::from_str(include_str!("fixtures/true-sky-hd-chat-app.json")).unwrap();
    assert_eq!(
        source["sourceRevision"],
        "dd2e03d9faf256342c84380ccb6fac83c06771ac"
    );
    let config = ZodiacConfiguration {
        reference: ZodiacReference::TrueSky,
        divisions: SignDivisions::Constellation,
        ophiuchus: Some(Ophiuchus::Enabled),
    };
    assert_eq!(source["positions"].as_array().unwrap().len(), 46);
    assert_eq!(source["houses"].as_array().unwrap().len(), 7);
    assert_eq!(source["projections"].as_array().unwrap().len(), 24);
    let sectors = zodiac_sectors(config).unwrap();
    assert_eq!(sectors.len(), 13);
    for (i, sector) in sectors.iter().enumerate() {
        let expected = &source["sectors"][i];
        assert_eq!(
            serde_json::to_value(sector.sign).unwrap().as_str().unwrap(),
            expected["sign"].as_str().unwrap().to_lowercase()
        );
        assert_eq!(sector.start_degrees, expected["start"].as_f64().unwrap());
    }
    for case in source["positions"].as_array().unwrap() {
        let actual = assign_sign(case["normalized"].as_f64().unwrap(), config).unwrap();
        assert_eq!(
            serde_json::to_value(actual.sign).unwrap().as_str().unwrap(),
            case["result"]["sign"].as_str().unwrap().to_lowercase()
        );
        assert!(
            (actual.degrees_in_sign - case["result"]["degreesInSign"].as_f64().unwrap()).abs()
                < 1e-12
        );
    }
    for case in source["houses"].as_array().unwrap() {
        let actual = whole_sign_cusps(case["ascendant"].as_f64().unwrap(), config).unwrap();
        for (i, cusp) in case["cusps"].as_array().unwrap().iter().enumerate() {
            assert_eq!(actual[i], cusp["longitude"].as_f64().unwrap());
        }
    }
    for case in source["projections"].as_array().unwrap() {
        let epoch = Epoch::from_str(case["utc"].as_str().unwrap()).unwrap();
        let projected = (case["longitude"].as_f64().unwrap() - true_sky_offset(epoch).unwrap())
            .rem_euclid(360.0);
        assert!(
            (projected - case["projected"].as_f64().unwrap()).abs() < 1e-12,
            "projection: {case}"
        );
    }
}

//! Mercator projection helpers (port of libs/geometry/mercator.cpp).
//!
//! Only the conversions the sandbox needs: mercator Y <-> latitude.
//! Longitude == mercator X, so no conversion is needed for it.

/// mercator::YToLat: lat = DegToRad(2 * atan(tanh(0.5 * DegToRad(y)))).
pub fn y_to_lat(y: f64) -> f64 {
    let rad = 0.5 * y.to_radians();
    (2.0 * rad.tanh().atan()).to_degrees()
}

/// mercator::LatToY with the latitude clamped to [-86, 86] and the result to
/// the mercator Y range [-180, 180].
pub fn lat_to_y(lat: f64) -> f64 {
    let sinx = lat.clamp(-86.0, 86.0).to_radians().sin();
    (0.5 * ((1.0 + sinx) / (1.0 - sinx)).ln())
        .to_degrees()
        .clamp(-180.0, 180.0)
}

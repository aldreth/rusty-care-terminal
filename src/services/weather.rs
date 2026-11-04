use serde_json::Value;

use crate::error::{AppError, Result};
use crate::model::{WeatherSnapshot, clean_text};
use crate::services::http::{self, encode_query};

/// Fetch current conditions + today's highs/lows via Open-Meteo (no API key).
pub fn fetch_weather(location: &str, celsius: bool) -> Result<WeatherSnapshot> {
    let (name, lat, lon) = resolve_location(location)?;
    let temp_unit = if celsius { "celsius" } else { "fahrenheit" };
    let wind_unit = if celsius { "kmh" } else { "mph" };

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
         &current=temperature_2m,weather_code,wind_speed_10m\
         &daily=weather_code,temperature_2m_max,temperature_2m_min\
         &temperature_unit={temp_unit}&wind_speed_unit={wind_unit}\
         &timezone=auto&forecast_days=1"
    );
    let json =
        http::get_json(&url).map_err(|e| AppError::Weather(format!("forecast for {name}: {e}")))?;
    snapshot_from_forecast(&json, name, celsius)
}

fn snapshot_from_forecast(json: &Value, name: String, celsius: bool) -> Result<WeatherSnapshot> {
    let current = &json["current"];
    let daily = &json["daily"];
    let temp = current["temperature_2m"]
        .as_f64()
        .ok_or_else(|| AppError::Weather("forecast had no current temperature".into()))?;
    let code = current["weather_code"]
        .as_u64()
        .and_then(|c| u16::try_from(c).ok());
    let wind = current["wind_speed_10m"].as_f64();
    let first_daily = |key: &str| {
        daily[key]
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_f64)
            .unwrap_or(temp)
    };
    let high = first_daily("temperature_2m_max");
    let low = first_daily("temperature_2m_min");
    let unit = if celsius { "°C" } else { "°F" };
    let wind_label = if celsius { "km/h" } else { "mph" };

    Ok(WeatherSnapshot {
        place: name,
        summary: code.map_or("Mixed skies", wmo_description).to_string(),
        temperature: format!("{temp:.0}{unit}"),
        high_low: format!("H {high:.0}{unit} · L {low:.0}{unit}"),
        wind: wind.map_or_else(String::new, |w| format!("wind {w:.0} {wind_label}")),
    })
}

fn resolve_location(raw: &str) -> Result<(String, f64, f64)> {
    let trimmed = raw.trim();
    if let Some((lat, lon)) = parse_lat_lon(trimmed) {
        return Ok((format!("{lat:.2},{lon:.2}"), lat, lon));
    }

    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        encode_query(trimmed)
    );
    let json = http::get_json(&url)
        .map_err(|e| AppError::Weather(format!("looking up '{trimmed}': {e}")))?;
    place_from_geocode(&json, trimmed)
}

fn place_from_geocode(json: &Value, query: &str) -> Result<(String, f64, f64)> {
    let first = json["results"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| AppError::Weather(format!("no place found for '{query}'")))?;

    let name = clean_text(first["name"].as_str().unwrap_or(query));
    let country = clean_text(first["country"].as_str().unwrap_or(""));
    let label = if country.is_empty() {
        name
    } else {
        format!("{name}, {country}")
    };
    let lat = first["latitude"]
        .as_f64()
        .ok_or_else(|| AppError::Weather("place lookup had no latitude".into()))?;
    let lon = first["longitude"]
        .as_f64()
        .ok_or_else(|| AppError::Weather("place lookup had no longitude".into()))?;
    Ok((label, lat, lon))
}

fn parse_lat_lon(raw: &str) -> Option<(f64, f64)> {
    let (a, b) = raw.split_once(',')?;
    let lat: f64 = a.trim().parse().ok()?;
    let lon: f64 = b.trim().parse().ok()?;
    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
        Some((lat, lon))
    } else {
        None
    }
}

fn wmo_description(code: u16) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm + hail",
        _ => "Mixed skies",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_lat_lon() {
        assert_eq!(parse_lat_lon("51.5,-0.12"), Some((51.5, -0.12)));
        assert!(parse_lat_lon("London").is_none());
        assert!(parse_lat_lon("91,0").is_none());
    }

    #[test]
    fn wmo_clear() {
        assert_eq!(wmo_description(0), "Clear sky");
    }

    #[test]
    fn builds_snapshot_from_forecast() {
        let forecast = json!({
            "current": { "temperature_2m": 11.6, "weather_code": 61, "wind_speed_10m": 14.2 },
            "daily": { "temperature_2m_max": [13.1], "temperature_2m_min": [7.4] }
        });
        let snap = snapshot_from_forecast(&forecast, "London, United Kingdom".into(), true)
            .expect("snapshot");
        assert_eq!(snap.temperature, "12°C");
        assert_eq!(snap.summary, "Rain");
        assert_eq!(snap.high_low, "H 13°C · L 7°C");
        assert_eq!(snap.wind, "wind 14 km/h");
    }

    #[test]
    fn missing_temperature_is_an_error_not_zero() {
        let forecast = json!({ "current": { "weather_code": 0 } });
        assert!(snapshot_from_forecast(&forecast, "x".into(), true).is_err());
    }

    #[test]
    fn geocode_place_is_cleaned() {
        let reply = json!({ "results": [{
            "name": "Lon\u{1b}[2Jdon", "country": "United Kingdom",
            "latitude": 51.5, "longitude": -0.12
        }]});
        let (label, _, _) = place_from_geocode(&reply, "London").unwrap();
        assert!(!label.contains('\u{1b}'));
        assert!(label.ends_with("United Kingdom"));
    }
}

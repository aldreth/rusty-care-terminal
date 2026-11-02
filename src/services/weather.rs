use std::time::Duration;

use chrono::Local;
use serde_json::Value;

use crate::error::{CareError, Result};
use crate::model::WeatherSnapshot;

const USER_AGENT: &str = concat!("rusty-care-terminal/", env!("CARGO_PKG_VERSION"));

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

    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| CareError::Feed(format!("weather client: {e}")))?;

    let json: Value = client
        .get(&url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| CareError::Feed(format!("weather forecast: {e}")))?;

    let current = &json["current"];
    let daily = &json["daily"];
    let temp = current["temperature_2m"].as_f64().unwrap_or(0.0);
    let code = current["weather_code"].as_u64().unwrap_or(0) as u16;
    let wind = current["wind_speed_10m"].as_f64().unwrap_or(0.0);
    let high = daily["temperature_2m_max"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_f64())
        .unwrap_or(temp);
    let low = daily["temperature_2m_min"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_f64())
        .unwrap_or(temp);
    let unit = if celsius { "°C" } else { "°F" };
    let wind_label = if celsius { "km/h" } else { "mph" };

    Ok(WeatherSnapshot {
        place: name,
        summary: wmo_description(code).to_string(),
        temperature: format!("{temp:.0}{unit}"),
        high_low: format!("H {high:.0}{unit} · L {low:.0}{unit}"),
        wind: format!("wind {wind:.0} {wind_label}"),
        fetched_at: Local::now(),
    })
}

fn resolve_location(raw: &str) -> Result<(String, f64, f64)> {
    let trimmed = raw.trim();
    if let Some((lat, lon)) = parse_lat_lon(trimmed) {
        return Ok((format!("{lat:.2},{lon:.2}"), lat, lon));
    }

    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlencoding_lite(trimmed)
    );
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| CareError::Feed(format!("geocode client: {e}")))?;

    let json: Value = client
        .get(&url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| CareError::Feed(format!("geocode: {e}")))?;

    let first = json["results"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| CareError::Feed(format!("no geocode results for '{trimmed}'")))?;

    let name = first["name"].as_str().unwrap_or(trimmed).to_string();
    let country = first["country"].as_str().unwrap_or("");
    let label = if country.is_empty() {
        name
    } else {
        format!("{name}, {country}")
    };
    let lat = first["latitude"]
        .as_f64()
        .ok_or_else(|| CareError::Feed("geocode missing latitude".into()))?;
    let lon = first["longitude"]
        .as_f64()
        .ok_or_else(|| CareError::Feed("geocode missing longitude".into()))?;
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

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
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

    #[test]
    fn parses_lat_lon() {
        assert_eq!(parse_lat_lon("51.5,-0.12"), Some((51.5, -0.12)));
        assert!(parse_lat_lon("London").is_none());
    }

    #[test]
    fn wmo_clear() {
        assert_eq!(wmo_description(0), "Clear sky");
    }
}

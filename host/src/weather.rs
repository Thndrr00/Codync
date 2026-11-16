//! Weather for a place name, from Open-Meteo (no key): a bot tool and a small API so a bot can
//! answer "what's the weather like" for the user's city, and remember it.

use crate::hub::Hub;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;

pub const METHODS: &[&str] = &["weather"];
const PLACE_KEY: &str = "weather-place.v1";
const TIMEOUT: Duration = Duration::from_secs(15);

pub fn tools() -> Value {
    json!([{
        "name": "weather",
        "description": "Current weather and a short forecast for a place. Without a place it uses the one saved on this computer. Powered by Open-Meteo.",
        "inputSchema": {"type": "object", "properties": {
            "place": {"type": "string", "description": "A city or place name, like \"Madrid\" or \"Porto, Portugal\"."},
            "save": {"type": "boolean", "description": "Remember this place for next time. Default false."},
        }},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    }])
}

pub async fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    ensure!(method == "weather", "unknown weather method {method}");
    forecast(hub, b).await
}

pub async fn tool_call(hub: &Hub, name: &str, args: &Value) -> Result<Value> {
    ensure!(name == "weather", "unknown weather tool: {name}");
    let answer = forecast(hub, args).await?;
    Ok(json!({"text": answer["text"]}))
}

async fn forecast(hub: &Hub, args: &Value) -> Result<Value> {
    let place = match args["place"].as_str().map(str::trim).filter(|p| !p.is_empty()) {
        Some(place) => place.to_owned(),
        None => hub
            .store
            .kv_get(PLACE_KEY)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("name a place once (the computer remembers it for next time)"))?,
    };
    ensure!(place.chars().count() <= 120, "that place name is too long");
    let found = geocode(&place).await?;
    if args["save"].as_bool() == Some(true) {
        let _ = hub.store.kv_set(PLACE_KEY, &place);
    }
    let (lat, lon) = (found["latitude"].as_f64(), found["longitude"].as_f64());
    let (Some(lat), Some(lon)) = (lat, lon) else { anyhow::bail!("the weather service didn't recognize that place") };
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&timezone=auto&forecast_days=3"
    );
    let response = crate::http().get(&url).timeout(TIMEOUT).send().await?;
    ensure!(response.status().is_success(), "the weather service answered {}", response.status());
    let answer: Value = response.json().await?;
    let current = &answer["current"];
    let temperature = current["temperature_2m"].as_f64();
    let feels = current["apparent_temperature"].as_f64();
    let code = current["weather_code"].as_u64().unwrap_or(0);
    let wind = current["wind_speed_10m"].as_f64().unwrap_or(0.0);
    let daily = &answer["daily"];
    let high = daily["temperature_2m_max"][0].as_f64();
    let low = daily["temperature_2m_min"][0].as_f64();
    let rain = daily["precipitation_probability_max"][0].as_u64();
    let where_ = [found["name"].as_str(), found["country"].as_str()]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let mut text = format!("{where_}: ");
    if let Some(temperature) = temperature {
        text.push_str(&format!("{temperature:.0}°C"));
        if let Some(feels) = feels {
            text.push_str(&format!(" (feels {feels:.0}°C)"));
        }
        text.push_str(&format!(", {}", describe(code)));
    }
    text.push_str(&format!(", wind {wind:.0} km/h."));
    if let (Some(high), Some(low)) = (high, low) {
        text.push_str(&format!(" Today {low:.0}–{high:.0}°C"));
        if let Some(rain) = rain {
            text.push_str(&format!(", rain {rain}%"));
        }
        text.push('.');
    }
    Ok(json!({
        "place": where_,
        "temperature": temperature,
        "feelsLike": feels,
        "condition": describe(code),
        "wind": wind,
        "high": high,
        "low": low,
        "rainChance": rain,
        "text": text,
    }))
}

async fn geocode(place: &str) -> Result<Value> {
    let response = crate::http()
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", place), ("count", "1"), ("language", "en"), ("format", "json")])
        .timeout(TIMEOUT)
        .send()
        .await?;
    ensure!(response.status().is_success(), "the place lookup answered {}", response.status());
    let answer: Value = response.json().await?;
    answer["results"]
        .as_array()
        .and_then(|results| results.first())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no place named \"{place}\""))
}

/// WMO weather codes from Open-Meteo, in a few words.
pub fn describe(code: u64) -> &'static str {
    match code {
        0 => "clear sky",
        1 => "mostly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 | 48 => "fog",
        51 | 53 | 55 => "drizzle",
        56 | 57 => "freezing drizzle",
        61 | 63 | 65 => "rain",
        66 | 67 => "freezing rain",
        71 | 73 | 75 => "snow",
        77 => "snow grains",
        80 | 81 | 82 => "rain showers",
        85 | 86 => "snow showers",
        95 => "thunderstorm",
        96 | 99 => "thunderstorm with hail",
        _ => "mixed conditions",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_codes_say_something_useful() {
        assert_eq!(describe(0), "clear sky");
        assert_eq!(describe(61), "rain");
        assert_eq!(describe(95), "thunderstorm");
        assert_eq!(describe(4_242), "mixed conditions");
    }
}

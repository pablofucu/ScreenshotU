//! Modelos de la respuesta de `play_histories` (ver `docs/nintendo-api.md`) y su paso a las filas
//! de [`super::store`]. Logica pura: sin red.

use serde::Deserialize;

use super::store::{DailyRow, TitleUpsert};
use super::normalize_title_id;

/// Fuente de los minutos diarios que vienen del historial de la cuenta.
pub const SOURCE_HISTORY: &str = "history";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayHistories {
    #[serde(default)]
    pub play_histories: Vec<HistoryTitle>,
    #[serde(default)]
    pub recent_play_histories: Vec<HistoryDay>,
}

/// Un juego con el total de toda la vida de la cuenta.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTitle {
    pub title_id: String,
    pub title_name: String,
    #[serde(default)]
    pub image_url: Option<String>,
    /// `HAC` (Switch 1) o `BEE` (Switch 2): la ultima consola, no la generacion del juego.
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub first_played_at: Option<String>,
    #[serde(default)]
    pub last_played_at: Option<String>,
    #[serde(default)]
    pub total_played_minutes: i64,
}

/// Un dia desglosado (la API da hoy y los 5 anteriores).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    /// `YYYY-MM-DDT00:00:00Z`.
    pub played_date: String,
    #[serde(default)]
    pub daily_play_histories: Vec<HistoryDayEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDayEntry {
    pub title_id: String,
    #[serde(default)]
    pub total_played_minutes: i64,
}

/// Interpreta el JSON del historial.
pub fn parse_play_histories(json: &str) -> anyhow::Result<PlayHistories> {
    Ok(serde_json::from_str(json)?)
}

/// Fecha ISO 8601 UTC a milisegundos desde epoch (`None` si no se entiende).
fn iso_to_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s.trim()).ok().map(|d| d.timestamp_millis())
}

impl PlayHistories {
    /// Un [`TitleUpsert`] por juego. Descarta los ids que no son de 16 hexadecimales.
    /// No filtra demos ni tiempos minimos: eso lo decide la importacion.
    pub fn title_upserts(&self) -> Vec<TitleUpsert> {
        self.play_histories
            .iter()
            .filter_map(|t| {
                Some(TitleUpsert {
                    nx_id: normalize_title_id(&t.title_id)?,
                    name: t.title_name.clone(),
                    image_url: t.image_url.clone(),
                    device_type: t.platform.clone(),
                    first_played_ms: t.first_played_at.as_deref().and_then(iso_to_ms),
                    last_played_ms: t.last_played_at.as_deref().and_then(iso_to_ms),
                    total_minutes: t.total_played_minutes,
                })
            })
            .collect()
    }

    /// Filas diarias de la fuente `history` (sin consola ni usuario). Descarta dias con fecha
    /// ilegible, ids invalidos y entradas de 0 minutos.
    pub fn daily_rows(&self) -> Vec<DailyRow> {
        let mut out = Vec::new();
        for d in &self.recent_play_histories {
            let Some(day) = d.played_date.get(..10).filter(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()) else {
                continue;
            };
            for e in &d.daily_play_histories {
                let Some(nx_id) = normalize_title_id(&e.title_id) else { continue };
                if e.total_played_minutes <= 0 {
                    continue;
                }
                out.push(DailyRow {
                    day: day.to_string(),
                    nx_id,
                    source: SOURCE_HISTORY.to_string(),
                    device_id: String::new(),
                    player_id: String::new(),
                    minutes: e.total_played_minutes,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/play_histories.json");

    #[test]
    fn parses_the_real_response() {
        let p = parse_play_histories(FIXTURE).unwrap();
        assert_eq!(p.play_histories.len(), 10);
        assert_eq!(p.recent_play_histories.len(), 6);
    }

    #[test]
    fn title_upserts_normalize_ids_and_dates() {
        let p = parse_play_histories(FIXTURE).unwrap();
        let ups = p.title_upserts();
        assert_eq!(ups.len(), 10);
        for u in &ups {
            assert_eq!(u.nx_id.len(), 16);
            assert_eq!(u.nx_id, u.nx_id.to_uppercase());
            assert!(u.first_played_ms.unwrap() <= u.last_played_ms.unwrap());
        }
        let oot = ups.iter().find(|u| u.name.contains("Ocarina")).unwrap();
        assert_eq!(oot.device_type.as_deref(), Some("BEE"));
        assert!(oot.total_minutes > 0);
    }

    #[test]
    fn switch2_platform_with_switch1_id_is_kept() {
        let p = parse_play_histories(FIXTURE).unwrap();
        let ups = p.title_upserts();
        let mina = ups.iter().find(|u| u.name.starts_with("Mina")).unwrap();
        assert!(mina.nx_id.starts_with("0100"));
        assert_eq!(mina.device_type.as_deref(), Some("BEE"));
    }

    #[test]
    fn demos_are_kept_for_the_import_to_filter() {
        let p = parse_play_histories(FIXTURE).unwrap();
        assert!(p.title_upserts().iter().any(|u| super::super::is_demo(&u.name)));
    }

    #[test]
    fn daily_rows_use_history_source_and_plain_day() {
        let p = parse_play_histories(FIXTURE).unwrap();
        let rows = p.daily_rows();
        assert!(!rows.is_empty());
        for r in &rows {
            assert_eq!(r.source, "history");
            assert_eq!(r.day.len(), 10);
            assert!(r.device_id.is_empty() && r.player_id.is_empty());
            assert!(r.minutes > 0);
        }
    }

    #[test]
    fn tolerates_missing_and_broken_data() {
        let p = parse_play_histories(
            r#"{"playHistories":[{"titleId":"zz","titleName":"X","totalPlayedMinutes":5},
                {"titleId":"0100ABCD00012000","titleName":"Y"}],
                "recentPlayHistories":[{"playedDate":"basura","dailyPlayHistories":[{"titleId":"0100ABCD00012000","totalPlayedMinutes":3}]},
                {"playedDate":"2026-11-07T00:00:00Z","dailyPlayHistories":[{"titleId":"0100abcd00012800","totalPlayedMinutes":0},{"titleId":"0100abcd00012800","totalPlayedMinutes":7}]}]}"#,
        )
        .unwrap();
        let ups = p.title_upserts();
        assert_eq!(ups.len(), 1);
        assert_eq!(ups[0].nx_id, "0100ABCD00012000");
        assert_eq!(ups[0].first_played_ms, None);
        let rows = p.daily_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].nx_id, "0100ABCD00012000");
        assert_eq!(rows[0].minutes, 7);
    }

    #[test]
    fn empty_object_is_valid() {
        let p = parse_play_histories("{}").unwrap();
        assert!(p.title_upserts().is_empty() && p.daily_rows().is_empty());
        assert!(parse_play_histories("no es json").is_err());
    }
}

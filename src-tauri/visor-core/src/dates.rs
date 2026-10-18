use regex::Regex;
use std::sync::OnceLock;

fn re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // AAAAMMDD con separadores opcionales, y opcionalmente HHMMSS detras.
        // Cubre Steam (20240115123456_1), Xbox/Game Bar (2024-01-15 12-34-56),
        // NVIDIA (Juego 2024.01.15 - 12.34.56.00), Switch (2024011512345600-HASH).
        Regex::new(
            r"(?:^|\D)((?:19|20)\d{2})[-._]?(0[1-9]|1[0-2])[-._]?(0[1-9]|[12]\d|3[01])(?:[-._ T]*([01]\d|2[0-3])[-._:]?([0-5]\d)[-._:]?([0-5]\d))?",
        )
        .unwrap()
    })
}

/// Dias desde 1970-01-01 para una fecha civil (algoritmo de Howard Hinnant).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Intenta sacar una fecha del nombre del archivo. Devuelve milisegundos "ingenuos" (se
/// interpretan como UTC; la interfaz tambien los muestra en UTC, asi que no hay desplazamiento).
pub fn parse_date_from_filename(name: &str) -> Option<i64> {
    let c = re().captures(name)?;
    let n = |i: usize| c.get(i).map(|m| m.as_str().parse::<i64>().unwrap());
    let (y, mo, d) = (n(1)?, n(2)?, n(3)?);
    let (h, mi, s) = (n(4).unwrap_or(0), n(5).unwrap_or(0), n(6).unwrap_or(0));
    Some(((days_from_civil(y, mo, d) * 24 + h) * 60 + mi) * 60_000 + s * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
        ((days_from_civil(y, mo, d) * 24 + h) * 60 + mi) * 60_000 + s * 1000
    }

    #[test]
    fn epoch_is_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
    }

    #[test]
    fn steam() {
        assert_eq!(parse_date_from_filename("20240115123456_1.jpg"), Some(ms(2024, 1, 15, 12, 34, 56)));
    }

    #[test]
    fn xbox_gamebar() {
        assert_eq!(
            parse_date_from_filename("Elden Ring 2024-03-09 21-05-44.png"),
            Some(ms(2024, 3, 9, 21, 5, 44))
        );
    }

    #[test]
    fn nvidia() {
        assert_eq!(
            parse_date_from_filename("Hades II 2025.11.02 - 08.15.30.12.png"),
            Some(ms(2025, 11, 2, 8, 15, 30))
        );
    }

    #[test]
    fn switch() {
        assert_eq!(
            parse_date_from_filename("2023103118452300-A1B2C3D4E5F60718.jpg"),
            Some(ms(2023, 10, 31, 18, 45, 23))
        );
    }

    #[test]
    fn date_only() {
        assert_eq!(parse_date_from_filename("captura_2022-12-25.png"), Some(ms(2022, 12, 25, 0, 0, 0)));
    }

    #[test]
    fn no_date() {
        assert_eq!(parse_date_from_filename("captura 7.png"), None);
        assert_eq!(parse_date_from_filename("img_99999999.png"), None);
        assert_eq!(parse_date_from_filename("20241345.png"), None); // mes 13
    }
}

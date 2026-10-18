//! Puntuacion (0..100) de un hilo de texto para recomendar el hilo de dialogo.
use std::collections::HashSet;

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}' | '\u{30FC}')
}

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{3005}')
}

fn is_jp_punct(c: char) -> bool {
    matches!(c, '、' | '。' | '「' | '」' | '『' | '』' | '！' | '？' | '…' | '―' | '・' | '（' | '）' | '【' | '】')
}

/// Puntuacion 0..100: mas alta cuanto mas parece texto de dialogo/narracion japonesa. Considera la
/// proporcion de kana/kanji, y penaliza caracteres duplicados, lineas de 1-2 caracteres y textos repetidos
/// (menus). Sin muestras = 0.
pub fn score(samples: &[String]) -> u8 {
    let samples: Vec<&str> = samples.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if samples.is_empty() {
        return 0;
    }
    let n = samples.len() as f64;
    let mut jp_sum = 0.0;
    let mut doubled = 0usize;
    let mut short = 0usize;
    let mut len_sum = 0usize;
    for s in &samples {
        let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
        let len = chars.len().max(1);
        len_sum += len;
        let jp = chars.iter().filter(|c| is_kana(**c) || is_kanji(**c)).count() as f64
            + 0.5 * chars.iter().filter(|c| is_jp_punct(**c)).count() as f64;
        jp_sum += (jp / len as f64).min(1.0);
        if len <= 2 {
            short += 1;
        }
        if super::filters::undouble(s).is_some() {
            doubled += 1;
        }
    }
    let unique = samples.iter().collect::<HashSet<_>>().len() as f64;
    let repeated = 1.0 - unique / n;
    let jp = jp_sum / n;
    let avg_len = len_sum as f64 / n;
    let len_factor = (avg_len / 8.0).clamp(0.2, 1.0);
    let v = jp
        * (1.0 - 0.9 * doubled as f64 / n)
        * (1.0 - 0.6 * short as f64 / n)
        * (1.0 - 0.7 * repeated)
        * len_factor;
    (v * 100.0).round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dialogo_alto_otros_bajos() {
        let buen = score(&v(&["美雪「明日、駅まで一緒に行ってくれないかな」", "窓の外では、雪が音もなく降り続いていた。", "「うん、いいよ。何時に待ち合わせる？」"]));
        let doble = score(&v(&["明明日日、、駅駅ままでで一一緒緒にに", "窓窓のの外外ででははああ雪雪がが降降るる"]));
        let menu = score(&v(&["はじめから", "つづきから", "はじめから", "つづきから", "はじめから"]));
        let corto = score(&v(&["は", "い", "あ", "う"]));
        let basura = score(&v(&["ABCD1234", "0x7FF0A", "kernel32.dll"]));
        assert!(buen >= 80, "{buen}");
        assert!(doble < buen / 2, "{doble}");
        assert!(menu < buen / 2, "{menu}");
        assert!(corto < 30, "{corto}");
        assert!(basura < 10, "{basura}");
        assert_eq!(score(&[]), 0);
    }
}

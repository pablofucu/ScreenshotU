//! Parte pura de la grabacion de audio de la lectura: mezcla a mono, recorte de silencios y WAV de 16 bits.
//! La captura real (WASAPI) y la voz sintetica viven en la aplicacion Tauri porque dependen de Windows.

/// Mezcla `channels` canales entrelazados a uno (media).
pub fn to_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples.chunks_exact(channels).map(|f| (f.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16).collect()
}

/// Rango `[desde, hasta)` de muestras que quedan tras quitar el silencio del principio y del final.
/// Se mira en ventanas de 10 ms: es sonido si alguna muestra supera `threshold` (de 32767). Se deja un margen
/// antes (`pad_before_ms`) y despues (`pad_after_ms`). `None` si todo es silencio.
pub fn trim_range(samples: &[i16], rate: u32, threshold: i16, pad_before_ms: u32, pad_after_ms: u32) -> Option<(usize, usize)> {
    let win = (rate as usize / 100).max(1);
    let loud = |w: &[i16]| w.iter().any(|&s| (s as i32).abs() > threshold as i32);
    let first = samples.chunks(win).position(loud)?;
    let last = samples.chunks(win).rposition(loud)?;
    let ms = |m: u32| rate as usize * m as usize / 1000;
    let from = (first * win).saturating_sub(ms(pad_before_ms));
    let to = ((last + 1) * win + ms(pad_after_ms)).min(samples.len());
    Some((from, to))
}

/// Recorte de silencios con los margenes habituales (120 ms antes, 250 ms despues).
pub fn trim_silence(samples: &[i16], rate: u32) -> Option<Vec<i16>> {
    trim_range(samples, rate, 350, 120, 250).map(|(a, b)| samples[a..b].to_vec())
}

/// Filtro biquad (RBJ) de segundo orden, forma directa I.
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    fn new(rate: u32, freq: f32, high: bool) -> Biquad {
        let w = 2.0 * std::f32::consts::PI * freq / rate.max(1) as f32;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let b = if high { [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0] } else { [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0] };
        Biquad { b: b.map(|v| v / a0), a: [-2.0 * cos / a0, (1.0 - alpha) / a0], x: [0.0; 2], y: [0.0; 2] }
    }

    fn run(&mut self, v: f32) -> f32 {
        let y = self.b[0] * v + self.b[1] * self.x[0] + self.b[2] * self.x[1] - self.a[0] * self.y[0] - self.a[1] * self.y[1];
        self.x = [v, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// Graves (60-250 Hz): un pasa-altos y dos pasa-bajos.
fn low_band(rate: u32) -> [Biquad; 3] {
    [Biquad::new(rate, 60.0, true), Biquad::new(rate, 250.0, false), Biquad::new(rate, 250.0, false)]
}

/// Media movil de `±k` ventanas.
fn smooth_avg(v: &[f32], k: usize) -> Vec<f32> {
    (0..v.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(k), (i + k + 1).min(v.len()));
            v[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect()
}

/// RMS 40 de 32767 (~ -58 dBFS) en la banda de la voz: por debajo es silencio.
const SOUND: f32 = 40.0 * 40.0;

/// Percentil `p` de una lista ya ordenada.
fn pct(sorted: &[f32], p: f32) -> f32 {
    sorted[((sorted.len() - 1) as f32 * p) as usize]
}

/// Lo que se calculo para decidir donde hay voz (para el informe de la ultima grabacion).
struct Detect {
    on: Vec<bool>,
    win: usize,
    base: f32,
    top: f32,
    thr: f32,
}

/// Ventanas (de 20 ms) con voz segun el timbre: la musica (piano, bajo, bateria...) lleva casi tanta energia en los
/// graves (60-250 Hz) como en la banda de la voz (300-3400 Hz), y la voz casi nada en los graves; donde habla alguien,
/// la banda de la voz sube muy por encima de los graves. La diferencia (en dB, media de 100 ms) de la musica sola es el
/// percentil bajo; es voz lo que la supera en `max(8 dB, la mitad del rango)` (como mucho 14). `None` si no hay
/// diferencia que valga (todo musica, todo voz o silencio).
fn timbre_voice(mono: &[i16], rate: u32) -> Option<Detect> {
    let win = (rate as usize / 50).max(1);
    let (mut fv, mut fl) = (VoiceBand::new(rate), low_band(rate));
    let (mut voice, mut low) = (Vec::new(), Vec::new());
    for w in mono.chunks(win) {
        let (mut pv, mut pl) = (0.0f32, 0.0f32);
        for &s in w {
            let v = fv.run(s as f32);
            let [a, b, c] = &mut fl;
            let l = c.run(b.run(a.run(s as f32)));
            pv += v * v;
            pl += l * l;
        }
        voice.push(pv / w.len() as f32);
        low.push(pl / w.len() as f32);
    }
    let (voice, low) = (smooth_avg(&voice, 2), smooth_avg(&low, 2));
    let diff: Vec<f32> = voice.iter().zip(&low).map(|(&v, &l)| 10.0 * (v / l.max(1.0)).log10()).collect();
    let mut q: Vec<f32> = diff.iter().zip(&voice).filter(|(_, &v)| v >= SOUND).map(|(&d, _)| d).collect();
    if q.len() < 10 {
        return None;
    }
    q.sort_by(|a, b| a.total_cmp(b));
    let (base, top) = (pct(&q, 0.25), pct(&q, 0.95));
    if top - base < 6.0 {
        return None;
    }
    let thr = base + (0.5 * (top - base)).clamp(8.0, 14.0);
    let on = diff.iter().zip(&voice).map(|(&d, &v)| v >= SOUND && d > thr).collect();
    Some(Detect { on, win, base, top, thr })
}

/// Recorte de una grabacion mono: se quitan las partes de solo musica (ver `timbre_voice`) y, si no distingue la voz,
/// solo los silencios del principio y del final.
pub fn trim_voice(samples: &[i16], rate: u32) -> Option<Vec<i16>> {
    trim_mono_report(samples, rate, &mut String::new())
}

fn trim_mono_report(mono: &[i16], rate: u32, report: &mut String) -> Option<Vec<i16>> {
    match timbre_voice(mono, rate) {
        Some(d) => {
            report.push_str(&format!("timbre (voz - graves): musica {:.1} dB, voz {:.1} dB, umbral {:.1} dB\n", d.base, d.top, d.thr));
            if let Some(out) = cut_spans(mono, &d, rate, report) {
                return Some(out);
            }
        }
        None => report.push_str("timbre: sin diferencia entre musica y voz\n"),
    }
    report.push_str("solo se quitan los silencios\n");
    let (a, b) = trim_range(mono, rate, 350, 120, 250)?;
    let mut out = mono[a..b].to_vec();
    fade_edges(&mut out, rate);
    Some(out)
}

/// Junta los tramos con voz de `d` (con margenes y fundidos). `None` si quedan menos de 0,3 s.
fn cut_spans(mono: &[i16], d: &Detect, rate: u32, report: &mut String) -> Option<Vec<i16>> {
    let spans = voice_spans(&d.on, d.win, mono.len(), rate, 150, 350);
    let total: usize = spans.iter().map(|(a, b)| b - a).sum();
    let s = |x: usize| x as f32 / rate as f32;
    let list: Vec<String> = spans.iter().map(|&(a, b)| format!("{:.2}-{:.2}", s(a), s(b))).collect();
    report.push_str(&format!("tramos con voz (s): {}\n", if list.is_empty() { "ninguno".into() } else { list.join(", ") }));
    if total < rate as usize * 3 / 10 {
        return None;
    }
    let mut out = Vec::with_capacity(total);
    for (a, b) in spans {
        let mut part = mono[a..b].to_vec();
        fade_edges(&mut part, rate);
        out.extend(part);
    }
    Some(out)
}

/// Banda de la voz (300-3400 Hz): dos pasa-altos y un pasa-bajos.
struct VoiceBand([Biquad; 3]);

impl VoiceBand {
    fn new(rate: u32) -> VoiceBand {
        VoiceBand([Biquad::new(rate, 300.0, true), Biquad::new(rate, 300.0, true), Biquad::new(rate, 3400.0, false)])
    }

    fn run(&mut self, v: f32) -> f32 {
        let [a, b, c] = &mut self.0;
        c.run(b.run(a.run(v)))
    }
}

/// Potencia media por ventanas de 20 ms, en la banda de la voz, del centro `(L+R)/2` y de los lados `(L-R)/2`.
fn mid_side_power(stereo: &[i16], rate: u32) -> (Vec<f32>, Vec<f32>, usize) {
    let win = (rate as usize / 50).max(1);
    let (mut fm, mut fs) = (VoiceBand::new(rate), VoiceBand::new(rate));
    let (mut mid, mut side) = (Vec::new(), Vec::new());
    for w in stereo.chunks(win * 2) {
        let (mut pm, mut ps, mut n) = (0.0f32, 0.0f32, 0usize);
        for f in w.chunks_exact(2) {
            let (l, r) = (f[0] as f32, f[1] as f32);
            let m = fm.run((l + r) * 0.5);
            let s = fs.run((l - r) * 0.5);
            pm += m * m;
            ps += s * s;
            n += 1;
        }
        let n = n.max(1) as f32;
        mid.push(pm / n);
        side.push(ps / n);
    }
    (mid, side, win)
}

/// Ventanas (de 20 ms) con voz segun el estereo: en las novelas la voz suena en el centro (es mono) y la musica es
/// estereo, asi que donde hay voz el centro crece mucho mas que los lados. La relacion centro/lados de la musica sola
/// es el percentil bajo; es voz lo que la supera en `max(2 dB, 35 % del rango)`. `None` si la grabacion es mono (o casi)
/// o no hay diferencia que valga: entonces no sirve este metodo.
fn center_voice(stereo: &[i16], rate: u32) -> Option<Detect> {
    let (mid, side, win) = mid_side_power(stereo, rate);
    let ratio_db = |m: f32, s: f32| 10.0 * (m / s.max(1.0)).log10();
    let mut q: Vec<f32> = mid.iter().zip(&side).filter(|(&m, _)| m >= SOUND).map(|(&m, &s)| ratio_db(m, s)).collect();
    if q.len() < 10 {
        return None;
    }
    q.sort_by(|a, b| a.total_cmp(b));
    let (base, top) = (pct(&q, 0.10), pct(&q, 0.95));
    // Lados 25 dB por debajo del centro casi siempre: es mono, no hay estereo con el que separar.
    if pct(&q, 0.5) > 25.0 || top - base < 3.0 {
        return None;
    }
    let thr = base + (0.35 * (top - base)).clamp(2.0, 6.0);
    let on = mid.iter().zip(&side).map(|(&m, &s)| m >= SOUND && ratio_db(m, s) > thr).collect();
    Some(Detect { on, win, base, top, thr })
}

/// Tramos `[desde, hasta)` (en muestras) con voz a partir de las ventanas marcadas: se suaviza (mayoria de 5 ventanas),
/// se unen los huecos de menos de 1 s (pausas de la propia frase), se quitan los tramos de menos de 120 ms y se anaden los margenes.
fn voice_spans(on: &[bool], win: usize, len: usize, rate: u32, pad_before_ms: u32, pad_after_ms: u32) -> Vec<(usize, usize)> {
    let ms = |m: u32| rate as usize * m as usize / 1000;
    let smooth: Vec<bool> = (0..on.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(2), (i + 3).min(on.len()));
            on[a..b].iter().filter(|&&x| x).count() * 2 > b - a
        })
        .collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < smooth.len() {
        if smooth[i] {
            let s = i;
            while i < smooth.len() && smooth[i] {
                i += 1;
            }
            match runs.last_mut() {
                Some(last) if (s - last.1) * win < ms(1000) => last.1 = i,
                _ => runs.push((s, i)),
            }
        } else {
            i += 1;
        }
    }
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (a, b) in runs.into_iter().filter(|(a, b)| (b - a) * win >= ms(120)) {
        let from = (a * win).saturating_sub(ms(pad_before_ms));
        let to = (b * win + ms(pad_after_ms)).min(len);
        match out.last_mut() {
            Some(last) if from <= last.1 => last.1 = to,
            _ => out.push((from, to)),
        }
    }
    out
}

/// Fundido de entrada y salida (15 ms) para que los cortes no chasqueen.
fn fade_edges(out: &mut [i16], rate: u32) {
    let fade = (rate as usize * 15 / 1000).min(out.len() / 2);
    let n = out.len();
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        out[i] = (out[i] as f32 * g) as i16;
        out[n - 1 - i] = (out[n - 1 - i] as f32 * g) as i16;
    }
}

/// Recorte de una grabacion estereo (entrelazada L,R): se quitan las partes de solo musica (al principio, al final y
/// los huecos largos de en medio) y se devuelve en mono. Primero con el estereo (la voz en el centro, ver
/// `center_voice`); si no sirve, por el timbre (ver `timbre_voice`).
pub fn trim_voice_stereo(stereo: &[i16], rate: u32) -> Option<Vec<i16>> {
    trim_voice_stereo_report(stereo, rate).0
}

/// Como `trim_voice_stereo`, y ademas un informe en texto de lo que decidio (para ajustar el filtro).
pub fn trim_voice_stereo_report(stereo: &[i16], rate: u32) -> (Option<Vec<i16>>, String) {
    let mono = to_mono(stereo, 2);
    let mut report = format!("grabacion: {:.2} s\n", secs(mono.len(), rate));
    match center_voice(stereo, rate) {
        Some(d) => {
            report.push_str(&format!("estereo (centro/lados): musica {:.1} dB, voz {:.1} dB, umbral {:.1} dB\n", d.base, d.top, d.thr));
            if let Some(out) = cut_spans(&mono, &d, rate, &mut report) {
                report.push_str(&format!("resultado: {:.2} s (estereo)\n", secs(out.len(), rate)));
                return (Some(out), report);
            }
        }
        None => report.push_str("estereo: no sirve (mono o sin diferencia)\n"),
    }
    let out = trim_mono_report(&mono, rate, &mut report);
    report.push_str(&format!("resultado: {}\n", out.as_ref().map(|o| format!("{:.2} s", secs(o.len(), rate))).unwrap_or("nada".into())));
    (out, report)
}

/// WAV PCM de 16 bits, mono.
pub fn wav_mono16(samples: &[i16], rate: u32) -> Vec<u8> {
    wav16(samples, rate, 1)
}

/// WAV PCM de 16 bits con `channels` canales entrelazados.
pub fn wav16(samples: &[i16], rate: u32, channels: u16) -> Vec<u8> {
    let block = 2 * channels;
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * block as u32).to_le_bytes());
    v.extend_from_slice(&block.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&s.to_le_bytes());
    }
    v
}

/// Duracion en segundos.
pub fn secs(samples: usize, rate: u32) -> f32 {
    samples as f32 / rate.max(1) as f32
}

/// Convierte bytes PCM de 16 bits little-endian en muestras (descarta un byte suelto final).
pub fn bytes_to_i16(b: &[u8]) -> Vec<i16> {
    b.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(n: usize, amp: i16) -> Vec<i16> {
        (0..n).map(|i| if i % 2 == 0 { amp } else { -amp }).collect()
    }

    #[test]
    fn mono_y_bytes() {
        assert_eq!(to_mono(&[100, 300, -50, 50], 2), vec![200, 0]);
        assert_eq!(to_mono(&[1, 2, 3], 1), vec![1, 2, 3]);
        assert_eq!(bytes_to_i16(&[0x01, 0x00, 0xFF, 0xFF, 0x7F]), vec![1, -1]);
    }

    #[test]
    fn recorta_silencios_con_margen() {
        let rate = 1000; // ventanas de 10 muestras
        let mut s = vec![0i16; 500];
        s.extend(tone(200, 5000));
        s.extend(vec![0i16; 800]);
        // sonido en 500..700; margen 100 ms antes (100 muestras) y 250 ms despues (250 muestras)
        let (a, b) = trim_range(&s, rate, 350, 100, 250).unwrap();
        assert_eq!((a, b), (400, 950));
        let t = trim_silence(&s, rate).unwrap();
        assert!(t.len() < s.len() && t.iter().any(|&x| x != 0));
        // margenes que se salen se limitan
        let short = tone(30, 5000);
        assert_eq!(trim_range(&short, rate, 350, 500, 500).unwrap(), (0, 30));
    }

    #[test]
    fn todo_silencio_es_none() {
        assert!(trim_range(&vec![0i16; 4410], 44100, 350, 120, 250).is_none());
        assert!(trim_range(&vec![100i16; 4410], 44100, 350, 120, 250).is_none()); // ruido bajo el umbral
        assert!(trim_silence(&[], 44100).is_none());
    }

    /// Musica (grave y constante) de fondo y, en medio, «voz» (1 kHz) bastante mas alta.
    fn music_voice(rate: u32) -> Vec<i16> {
        let n = rate as usize * 4;
        (0..n)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let music = 1500.0 * (2.0 * std::f32::consts::PI * 110.0 * t).sin() + 600.0 * (2.0 * std::f32::consts::PI * 880.0 * t).sin();
                let voice = if (1.5..2.5).contains(&t) { 6000.0 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin() } else { 0.0 };
                (music + voice) as i16
            })
            .collect()
    }

    #[test]
    fn recorta_a_la_voz_con_musica() {
        let rate = 16_000;
        let s = music_voice(rate);
        let t = trim_voice(&s, rate).unwrap();
        let secs = t.len() as f32 / rate as f32;
        // 1 s de voz + 0,5 s de margenes
        assert!((secs - 1.5).abs() < 0.15, "{secs}");
        // solo musica, sin voz: no se recorta a nada raro (como mucho, los silencios)
        let only: Vec<i16> = (0..rate as usize * 2).map(|i| (1500.0 * (i as f32 * 0.043).sin()) as i16).collect();
        assert_eq!(trim_voice(&only, rate).map(|v| v.len()), Some(only.len()));
        assert!(trim_voice(&vec![0i16; 8000], rate).is_none());
    }

    /// Estereo entrelazado: musica distinta en cada canal (ancha) y «voz» igual en los dos (centro) en los tramos `voz`.
    fn stereo_music_voice(rate: u32, secs: f32, voz: &[(f32, f32)], music_amp: f32, voice_amp: f32) -> Vec<i16> {
        let n = (rate as f32 * secs) as usize;
        let pi2 = 2.0 * std::f32::consts::PI;
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = i as f32 / rate as f32;
            let l = music_amp * ((pi2 * 523.0 * t).sin() + 0.5 * (pi2 * 1320.0 * t).sin());
            let r = music_amp * ((pi2 * 659.0 * t).sin() + 0.5 * (pi2 * 990.0 * t).sin());
            // voz modulada como silabas (~4 Hz)
            let v = if voz.iter().any(|&(a, b)| (a..b).contains(&t)) { voice_amp * (pi2 * 800.0 * t).sin() * (0.6 + 0.4 * (pi2 * 4.0 * t).sin()) } else { 0.0 };
            out.push((l + v) as i16);
            out.push((r + v) as i16);
        }
        out
    }

    #[test]
    fn estereo_quita_la_musica_de_los_bordes() {
        let rate = 16_000;
        let s = stereo_music_voice(rate, 5.0, &[(2.0, 3.0)], 2500.0, 5000.0);
        let t = trim_voice_stereo(&s, rate).unwrap();
        let secs = t.len() as f32 / rate as f32;
        // 1 s de voz + 0,5 s de margenes
        assert!((secs - 1.5).abs() < 0.15, "{secs}");
    }

    #[test]
    fn estereo_con_musica_mas_alta_que_la_voz() {
        let rate = 16_000;
        let s = stereo_music_voice(rate, 5.0, &[(1.5, 3.0)], 4000.0, 3000.0);
        let secs = trim_voice_stereo(&s, rate).unwrap().len() as f32 / rate as f32;
        assert!((secs - 2.0).abs() < 0.2, "{secs}");
    }

    #[test]
    fn estereo_quita_los_huecos_largos_de_musica() {
        let rate = 16_000;
        let s = stereo_music_voice(rate, 7.0, &[(1.0, 2.0), (4.5, 5.5)], 2500.0, 5000.0);
        let secs = trim_voice_stereo(&s, rate).unwrap().len() as f32 / rate as f32;
        // dos tramos de 1 s, cada uno con 0,5 s de margenes; los 2,5 s de musica del medio fuera
        assert!((secs - 3.0).abs() < 0.25, "{secs}");
    }

    #[test]
    fn mono_en_dos_canales_usa_la_energia() {
        let rate = 16_000;
        let m = music_voice(rate);
        let s: Vec<i16> = m.iter().flat_map(|&x| [x, x]).collect();
        assert!(center_voice(&s, rate).is_none());
        assert_eq!(trim_voice_stereo(&s, rate), trim_voice(&m, rate));
        assert_eq!(&wav16(&[1, 2], 8000, 2)[22..24], &2u16.to_le_bytes());
    }

    /// Como la de la ultima grabacion real: notas de piano con bajo que decaen cada 0,75 s y una voz (armonicos de
    /// 200 Hz, silabas a 4 Hz) igual de fuerte que la musica entre 2 y 4,5 s.
    #[test]
    fn mono_voz_tan_alta_como_la_musica() {
        let rate = 16_000;
        let pi2 = 2.0 * std::f32::consts::PI;
        let s: Vec<i16> = (0..rate as usize * 7)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let env = (-(t % 0.75) * 5.0).exp();
                let music = 3000.0 * env * ((pi2 * 98.0 * t).sin() + 0.8 * (pi2 * 196.0 * t).sin() + 0.5 * (pi2 * 392.0 * t).sin());
                let voice = if (2.0..4.5).contains(&t) {
                    let syl = 0.55 + 0.45 * (pi2 * 4.0 * t).sin();
                    (2..12).map(|h| (pi2 * 200.0 * h as f32 * t).sin() / h as f32).sum::<f32>() * 1400.0 * syl
                } else {
                    0.0
                };
                (music + voice).clamp(-32000.0, 32000.0) as i16
            })
            .collect();
        let st: Vec<i16> = s.iter().flat_map(|&x| [x, x]).collect();
        let (t, report) = trim_voice_stereo_report(&st, rate);
        let secs = t.unwrap().len() as f32 / rate as f32;
        assert!((secs - 3.0).abs() < 0.3, "{secs}
{report}");
    }

    #[test]
    fn wav_valido() {
        let w = wav_mono16(&[1, -2, 3], 44100);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()) as usize, w.len() - 8);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 6);
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), -2);
        assert!((secs(44100, 44100) - 1.0).abs() < 1e-6);
    }
}

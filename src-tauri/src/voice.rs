//! Voz sintetica de Windows (`Windows.Media.SpeechSynthesis`) para las frases sin audio grabado. Hace falta una voz
//! japonesa instalada (Configuracion › Hora e idioma › Idioma › Japones › Voz). Devuelve un WAV.

#[cfg(windows)]
pub fn synthesize(text: &str) -> Result<Vec<u8>, String> {
    use windows::core::HSTRING;
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;
    use windows::Storage::Streams::DataReader;
    let _ = wasapi::initialize_mta(); // COM/WinRT en este hilo
    let text = text.trim();
    if text.is_empty() {
        return Err("La frase está vacía.".into());
    }
    let e = |what: &str, e: windows::core::Error| format!("Voz sintética ({what}): {e}");
    let synth = SpeechSynthesizer::new().map_err(|x| e("crear", x))?;
    let voices = SpeechSynthesizer::AllVoices().map_err(|x| e("voces", x))?;
    let ja = voices.into_iter().find(|v| v.Language().map(|l| l.to_string().to_lowercase().starts_with("ja")).unwrap_or(false));
    let Some(voice) = ja else {
        return Err("No hay ninguna voz japonesa instalada en Windows: añade el idioma japonés con su voz (Configuración › Hora e idioma).".into());
    };
    synth.SetVoice(&voice).map_err(|x| e("elegir voz", x))?;
    let stream = synth
        .SynthesizeTextToStreamAsync(&HSTRING::from(text))
        .map_err(|x| e("sintetizar", x))?
        .join()
        .map_err(|x| e("sintetizar", x))?;
    let size = stream.Size().map_err(|x| e("tamaño", x))? as u32;
    let input = stream.GetInputStreamAt(0).map_err(|x| e("leer", x))?;
    let reader = DataReader::CreateDataReader(&input).map_err(|x| e("leer", x))?;
    reader.LoadAsync(size).map_err(|x| e("leer", x))?.join().map_err(|x| e("leer", x))?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).map_err(|x| e("leer", x))?;
    Ok(buf)
}

#[cfg(not(windows))]
pub fn synthesize(_text: &str) -> Result<Vec<u8>, String> {
    Err("La voz sintética solo está disponible en Windows".into())
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCfg {
    pub id: String,
    /// Nombre que se muestra en la pantalla de inicio.
    pub name: String,
    /// Nombre de la carpeta dentro de la raiz (sin distinguir mayusculas).
    pub folder: String,
    /// Indice (0..6) del acento arcoiris de la plataforma.
    pub accent: u8,
    /// Glifo: "pc", "switch", "vn" o "jvn".
    pub icon: String,
}

/// Que campo del tipo de nota de Anki recibe cada dato ("" = no se envia).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AnkiMap {
    pub word: String,
    pub reading: String,
    pub meaning: String,
    pub sentence: String,
    pub translation: String,
    pub shot: String,
    pub audio: String,
}

impl Default for AnkiMap {
    fn default() -> Self {
        AnkiMap {
            word: "Word".into(),
            reading: "WordReading".into(),
            meaning: "PrimaryDefinition".into(),
            sentence: "Sentence".into(),
            translation: "SentenceTranslation".into(),
            shot: "Picture".into(),
            audio: "SentenceAudio".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub root: Option<String>,
    pub achievements_card_variant: u8,
    /// "grid" | "carousel"
    pub titles_view: String,
    /// Cuadricula de capturas: "grid" (continua) | "days" (agrupada por dias)
    pub shots_view: String,
    /// Orden inicial de toda lista de capturas: "recent" (mas recientes primero) | "oldest" (mas antiguas primero)
    pub shots_order: String,
    /// "art" (del juego, por defecto) | "rainbow" | "fixed"
    pub accent_mode: String,
    pub accent_fixed: u8,
    /// Tema de la interfaz: "light" | "dark" (Noche ahumada)
    pub theme: String,
    /// Juegos fijados en "En marcha" de Mi lista (ids de titulo)
    pub pinned: Vec<String>,
    pub sound: bool,
    pub platforms: Vec<PlatformCfg>,
    /// Atajo global de captura ("" = desactivado). Ej.: "F12", "Ctrl+F12".
    pub capture_hotkey: String,
    /// Combinacion de mando `boton+boton` (ver `capture::PAD_BUTTONS`), "off" = desactivada.
    pub capture_combo: String,
    /// Boton del raton que captura (ver `capture::MOUSE_BUTTONS`), "" = desactivado.
    pub capture_mouse: String,
    /// Con Magpie escalando un juego: "scaled" guarda lo que se ve (escalado), "source" la imagen original.
    pub capture_magpie: String,
    /// Sonido al guardar una captura.
    pub capture_sound: bool,
    /// Miniatura de la captura unos instantes sobre el juego.
    pub capture_peek: bool,
    /// Capturar solo dentro de un juego (lanzado desde el visor, conocido o en pantalla completa); fuera, no.
    pub capture_only_games: bool,
    /// Vibracion corta del mando cuando se captura con el.
    pub capture_rumble: bool,
    /// Ruta de `Magpie.exe` ("" = buscarlo solo).
    pub magpie_exe: String,
    /// Mostrar atenuados los juegos ocultos en las listas.
    pub show_hidden: bool,
    /// Clave de la API de SteamGridDB ("" = sin conexion).
    pub sgdb_key: String,
    /// Descargar solos los grafismos que falten.
    pub sgdb_auto: bool,
    /// Clave de DeepSeek para traducir y explicar frases ("" = sin IA). Solo la usa la app, nunca la pagina.
    pub deepseek_key: String,
    pub deepseek_model: String,
    /// AnkiConnect: URL, mazo y tipo de nota ("" = el propio de la app, «Visor · Japonés») y mapeo de campos
    /// (solo para tipos de nota ajenos).
    pub anki_url: String,
    pub anki_deck: String,
    pub anki_model: String,
    pub anki_map: AnkiMap,
    /// Diseño de la tarjeta del tipo de nota propio (ver `jp::anki_tpl::STYLES`).
    pub anki_style: String,
    /// Abrir Anki (minimizado) al lanzar una novela japonesa si no esta abierto.
    pub anki_autostart: bool,
    /// Ruta de `anki.exe` ("" = buscarlo solo).
    pub anki_exe: String,
    /// Atajo global que empieza/para la grabacion del audio del juego ("" = desactivado).
    pub record_hotkey: String,
    /// Combinacion de mando que cambia la salida de audio de Windows (como `capture_combo`), "off" = desactivada.
    pub audio_combo: String,
    /// Atajo global que cambia a la siguiente salida de audio ("" = desactivado).
    pub audio_hotkey: String,
    /// Copia automatica de `visor.db` al arrancar si la ultima tiene 7 dias o mas (se conservan 5).
    pub auto_backup: bool,
    /// Copia las partidas guardadas de un juego al cerrarlo (si cambiaron desde la ultima copia).
    pub saves_auto: bool,
    /// Carpeta de las copias de partidas ("" = `<config>/partidas`).
    pub saves_dir: String,
    /// Meta diaria de lectura (minutos) de las novelas visuales y de las japonesas (anillos de Inicio).
    pub goal_vn: u32,
    pub goal_jvn: u32,
    /// Al arrancar, abrir el visor en el monitor donde esta el cursor (el del icono pulsado) en vez de en el ultimo usado.
    pub open_on_cursor: bool,
    /// Servidor para la app movil (ver `remote` y `docs/api-movil.md`): activado, puerto y codigo de emparejamiento
    /// (se genera la primera vez que se activa).
    pub remote_enabled: bool,
    pub remote_port: u16,
    pub remote_token: String,
    /// Orden de la lista de juegos por categoria (id de plataforma -> "last" | "az" | "color"; ausente = "last").
    pub titles_order: std::collections::BTreeMap<String, String>,
    /// Tamano de las portadas de la lista de juegos: 0 = 5 por fila (por defecto) .. 3 = 8 por fila.
    pub titles_zoom: u8,
    /// Estilo de la pestana de las colecciones: "aurora" (por defecto) | "multi" (multicolor armonizada).
    pub collections_style: String,
    /// «Mi lista · Juegos parados»: avisar de los juegos en «Jugando» que llevan tiempo sin jugarse.
    pub list_stalled: bool,
    /// Dias sin jugar para considerar un juego parado (7 a 180).
    pub list_stalled_days: u32,
    /// Que hacer con ellos: "ask" (preguntar) | "pending" | "dropped".
    pub list_stalled_action: String,
    /// Al lanzar un pendiente desde el visor, pasa a «Jugando».
    pub list_auto_play: bool,
    /// Estadisticas de juego: contar clics, rueda y distancia del cursor durante las partidas (solo contadores).
    pub input_mouse: bool,
    /// Lo mismo con las pulsaciones de teclado. Solo se cuentan, nunca se guarda que tecla; desactivado por defecto.
    pub input_keys: bool,
}

pub fn default_platforms() -> Vec<PlatformCfg> {
    let p = |id: &str, name: &str, folder: &str, accent: u8, icon: &str| PlatformCfg {
        id: id.into(),
        name: name.into(),
        folder: folder.into(),
        accent,
        icon: icon.into(),
    };
    vec![
        p("pc", "Juegos PC", "PC", 4, "pc"),
        p("switch", "Juegos Switch", "Switch", 0, "switch"),
        p("vn", "Novelas visuales", "Novelas visuales", 6, "vn"),
        novels_platform(),
    ]
}

/// Id de la categoria de novelas japonesas: sus juegos se lanzan con texthook y pagina de lectura.
pub const NOVELS_PLATFORM: &str = "jvn";

/// Categoria «Novelas japonesas» (texthook + lectura con diccionario).
pub fn novels_platform() -> PlatformCfg {
    PlatformCfg {
        id: NOVELS_PLATFORM.into(),
        name: "Novelas japonesas".into(),
        folder: "Novelas japonesas".into(),
        accent: 5,
        icon: "jvn".into(),
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            root: None,
            achievements_card_variant: 1,
            titles_view: "grid".into(),
            shots_view: "grid".into(),
            shots_order: "recent".into(),
            accent_mode: "art".into(),
            accent_fixed: 4,
            theme: "light".into(),
            pinned: Vec::new(),
            sound: false,
            platforms: default_platforms(),
            capture_hotkey: "F12".into(),
            capture_combo: "back+rb".into(),
            capture_mouse: String::new(),
            capture_magpie: "scaled".into(),
            capture_sound: true,
            capture_peek: true,
            capture_only_games: true,
            capture_rumble: true,
            magpie_exe: String::new(),
            show_hidden: false,
            sgdb_key: String::new(),
            sgdb_auto: true,
            deepseek_key: String::new(),
            deepseek_model: "deepseek-chat".into(),
            anki_url: "http://127.0.0.1:8765".into(),
            anki_deck: "Japonés::Novelas".into(),
            anki_model: String::new(),
            anki_map: AnkiMap::default(),
            anki_style: "limpio".into(),
            anki_autostart: true,
            anki_exe: String::new(),
            record_hotkey: "F9".into(),
            audio_combo: "start+rb".into(),
            audio_hotkey: String::new(),
            auto_backup: true,
            saves_auto: true,
            saves_dir: String::new(),
            goal_vn: 60,
            goal_jvn: 30,
            open_on_cursor: false,
            remote_enabled: false,
            remote_port: crate::remote::DEFAULT_PORT,
            remote_token: String::new(),
            titles_order: Default::default(),
            titles_zoom: 0,
            collections_style: "aurora".into(),
            list_stalled: true,
            list_stalled_days: 30,
            list_stalled_action: "ask".into(),
            list_auto_play: true,
            input_mouse: true,
            input_keys: false,
        }
    }
}

impl Config {
    /// Carga la configuracion; si no existe o esta rota devuelve la de por defecto.
    pub fn load(path: &Path) -> Config {
        let mut c: Config = match fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                // Ilegible: se aparta (en vez de pisarla con la de por defecto al guardar) y se sigue con los valores por defecto.
                eprintln!("config.json ilegible ({e}); se guarda como config.json.corrupta");
                let _ = fs::rename(path, path.with_extension("json.corrupta"));
                Config::default()
            }),
            Err(_) => Config::default(),
        };
        if c.platforms.is_empty() {
            c.platforms = default_platforms();
        }
        // Configuraciones anteriores a la categoria de novelas japonesas: se anade al final.
        if !c.platforms.iter().any(|p| p.id == NOVELS_PLATFORM) {
            c.platforms.push(novels_platform());
        }
        c.accent_fixed = c.accent_fixed.min(6);
        c.goal_vn = c.goal_vn.clamp(5, 600);
        c.goal_jvn = c.goal_jvn.clamp(5, 600);
        c.remote_port = c.remote_port.max(1024);
        c.remote_token = c.remote_token.trim().to_string();
        c.titles_zoom = c.titles_zoom.min(3);
        if !["aurora", "multi"].contains(&c.collections_style.as_str()) {
            c.collections_style = "aurora".into();
        }
        c.list_stalled_days = c.list_stalled_days.clamp(7, 180);
        if !["ask", "pending", "dropped"].contains(&c.list_stalled_action.as_str()) {
            c.list_stalled_action = "ask".into();
        }
        if !["recent", "oldest"].contains(&c.shots_order.as_str()) {
            c.shots_order = "recent".into();
        }
        if !["art", "rainbow", "fixed"].contains(&c.accent_mode.as_str()) {
            c.accent_mode = "art".into();
        }
        if c.capture_combo != "off" && crate::capture::combo_mask(&c.capture_combo) == 0 {
            c.capture_combo = "back+rb".into();
        }
        if c.audio_combo != "off" && crate::capture::combo_mask(&c.audio_combo) == 0 {
            c.audio_combo = "start+rb".into();
        }
        if !c.capture_mouse.is_empty() && crate::capture::mouse_button_index(&c.capture_mouse) == 0 {
            c.capture_mouse.clear();
        }
        if !["scaled", "source"].contains(&c.capture_magpie.as_str()) {
            c.capture_magpie = "scaled".into();
        }
        c.sgdb_key = c.sgdb_key.trim().to_string();
        c.deepseek_key = c.deepseek_key.trim().to_string();
        c.deepseek_model = c.deepseek_model.trim().to_string();
        c.anki_url = c.anki_url.trim().trim_end_matches('/').to_string();
        if c.anki_url.is_empty() {
            c.anki_url = "http://127.0.0.1:8765".into();
        }
        c.anki_deck = c.anki_deck.trim().to_string();
        c.anki_model = c.anki_model.trim().to_string();
        if !crate::jp::anki_tpl::STYLES.iter().any(|s| s.0 == c.anki_style) {
            c.anki_style = "limpio".into();
        }
        c.anki_exe = c.anki_exe.trim().to_string();
        if c.deepseek_model.is_empty() {
            c.deepseek_model = "deepseek-chat".into();
        }
        c
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_atomic(path, serde_json::to_string_pretty(self)?.as_bytes())
    }
}

pub fn load_favorites(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_favorites(path: &Path, favs: &[String]) -> Result<()> {
    write_atomic(path, serde_json::to_string_pretty(favs)?.as_bytes())
}

/// Escribe en un temporal y renombra, para no dejar JSON a medias si se corta la luz.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_with_four_platforms() {
        let c = Config::load(Path::new("/no/existe/config.json"));
        assert_eq!(c.platforms.len(), 4);
        assert_eq!(c.titles_view, "grid");
        assert!(c.root.is_none());
    }

    #[test]
    fn broken_file_is_set_aside_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        fs::write(&p, r#"{"root":"D:\Caps","sound":tr"#).unwrap();
        let c = Config::load(&p);
        assert!(c.root.is_none());
        assert!(!p.exists());
        assert!(d.path().join("config.json.corrupta").exists());
    }

    #[test]
    fn roundtrip_and_partial_json() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        fs::write(&p, r#"{"root":"D:\\Caps","sound":true}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!(c.root.as_deref(), Some("D:\\Caps"));
        assert!(c.sound);
        assert_eq!(c.platforms.len(), 4);
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p), c);
    }

    #[test]
    fn my_list_settings_default_on_and_are_clamped() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config::load(&p);
        assert_eq!((c.list_stalled, c.list_stalled_days, c.list_stalled_action.as_str(), c.list_auto_play), (true, 30, "ask", true));
        fs::write(&p, r#"{"listStalledDays":1,"listStalledAction":"raro","listAutoPlay":false}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.list_stalled_days, c.list_stalled_action.as_str(), c.list_auto_play, c.list_stalled), (7, "ask", false, true));
        fs::write(&p, r#"{"listStalledDays":999,"listStalledAction":"dropped"}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.list_stalled_days, c.list_stalled_action.as_str()), (180, "dropped"));
    }

    #[test]
    fn old_config_gets_novels_platform() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        fs::write(&p, r#"{"platforms":[{"id":"pc","name":"PC","folder":"PC","accent":4,"icon":"pc"}]}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!(c.platforms.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["pc", "jvn"]);
    }

    #[test]
    fn accent_mode_defaults_to_art_and_old_values_survive() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        assert_eq!(Config::load(&p).accent_mode, "art", "por defecto, el acento sale del juego");
        fs::write(&p, r#"{"accentMode":"rainbow","accentFixed":9}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.accent_mode.as_str(), c.accent_fixed, c.show_hidden), ("rainbow", 6, false));
        fs::write(&p, r#"{"accentMode":"raro"}"#).unwrap();
        assert_eq!(Config::load(&p).accent_mode, "art");
    }

    #[test]
    fn collections_style_defaults_to_aurora_and_rejects_unknown() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        assert_eq!(Config::load(&p).collections_style, "aurora");
        fs::write(&p, r#"{"collectionsStyle":"multi"}"#).unwrap();
        assert_eq!(Config::load(&p).collections_style, "multi");
        fs::write(&p, r#"{"collectionsStyle":"raro"}"#).unwrap();
        assert_eq!(Config::load(&p).collections_style, "aurora");
    }

    #[test]
    fn shots_order_defaults_to_recent_and_rejects_unknown() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        assert_eq!(Config::load(&p).shots_order, "recent");
        fs::write(&p, r#"{"shotsOrder":"oldest"}"#).unwrap();
        assert_eq!(Config::load(&p).shots_order, "oldest");
        fs::write(&p, r#"{"shotsOrder":"raro"}"#).unwrap();
        assert_eq!(Config::load(&p).shots_order, "recent");
    }

    #[test]
    fn capture_bindings_are_validated() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config::load(&p);
        assert_eq!((c.capture_combo.as_str(), c.capture_mouse.as_str(), c.capture_magpie.as_str()), ("back+rb", "", "scaled"));
        fs::write(&p, r#"{"captureCombo":"lt+rt+y","captureMouse":"x2","captureMagpie":"source"}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.capture_combo.as_str(), c.capture_mouse.as_str(), c.capture_magpie.as_str()), ("lt+rt+y", "x2", "source"));
        fs::write(&p, r#"{"captureCombo":"zz","captureMouse":"left","captureMagpie":"?"}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.capture_combo.as_str(), c.capture_mouse.as_str(), c.capture_magpie.as_str()), ("back+rb", "", "scaled"));
        fs::write(&p, r#"{"captureCombo":"off"}"#).unwrap();
        assert_eq!(Config::load(&p).capture_combo, "off");
    }

    #[test]
    fn deepseek_defaults_and_trim() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config::load(&p);
        assert_eq!((c.deepseek_key.as_str(), c.deepseek_model.as_str()), ("", "deepseek-chat"));
        fs::write(&p, r#"{"deepseekKey":"  sk-abc ","deepseekModel":"  "}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.deepseek_key.as_str(), c.deepseek_model.as_str()), ("sk-abc", "deepseek-chat"));
    }

    #[test]
    fn anki_defaults_and_partial_map() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config::load(&p);
        assert_eq!((c.anki_url.as_str(), c.anki_map.shot.as_str()), ("http://127.0.0.1:8765", "Picture"));
        assert_eq!(c.anki_deck, "Japonés::Novelas");
        fs::write(&p, r#"{"ankiUrl":" http://localhost:8765/ ","ankiMap":{"word":"Expression"}}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.anki_url.as_str(), c.anki_map.word.as_str(), c.anki_map.audio.as_str()), ("http://localhost:8765", "Expression", "SentenceAudio"));
        fs::write(&p, r#"{"ankiUrl":"  "}"#).unwrap();
        assert_eq!(Config::load(&p).anki_url, "http://127.0.0.1:8765");
    }

    #[test]
    fn record_hotkey_default() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        assert_eq!(Config::load(&p).record_hotkey, "F9");
        fs::write(&p, r#"{"recordHotkey":""}"#).unwrap();
        assert_eq!(Config::load(&p).record_hotkey, "");
    }

    #[test]
    fn remote_defaults_and_port_clamp() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config::load(&p);
        assert_eq!((c.remote_enabled, c.remote_port, c.remote_token.as_str()), (false, 47811, ""));
        fs::write(&p, r#"{"remoteEnabled":true,"remotePort":80,"remoteToken":" ABCD-EFGH "}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!((c.remote_enabled, c.remote_port, c.remote_token.as_str()), (true, 1024, "ABCD-EFGH"));
    }

    #[test]
    fn favorites_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/f.json");
        save_favorites(&p, &["a.png".into(), "b.png".into()]).unwrap();
        assert_eq!(load_favorites(&p), vec!["a.png", "b.png"]);
    }
}

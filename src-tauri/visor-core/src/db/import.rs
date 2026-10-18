//! Importacion unica de los JSON antiguos a la base. Se hace una sola vez (meta `json_import`), dentro
//! de una transaccion, y **nunca borra ni modifica los JSON**: quedan como respaldo.
//! Un JSON ausente se salta; uno roto se registra (`eprintln`) y se salta, sin perder los demas. Dentro
//! de una lista o un mapa, los elementos ilegibles se saltan uno a uno.
use super::data::*;
use crate::games::PlayStat;
use crate::jp::phrases::Phrase;
use crate::jp::NovelPrefs;
use crate::sagas::{self, Saga};
use crate::sgdb::SgdbEntry;
use crate::userdata::{HiddenEntry, TitlePref};
use anyhow::Result;
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Clave de `meta` que marca que la importacion ya se hizo (su valor: ms desde epoch).
pub const META_IMPORTED: &str = "json_import";
/// Clave de `meta` con el resumen de lo importado.
pub const META_REPORT: &str = "json_import_report";

/// Resultado de leer un JSON.
enum Json {
    Missing,
    Broken,
    Value(Value),
}

fn read_json(path: &Path) -> Json {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Json::Missing,
        Err(e) => {
            eprintln!("Importacion: no se pudo leer {}: {e}", path.display());
            return Json::Broken;
        }
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => Json::Value(v),
        Err(e) => {
            eprintln!("Importacion: {} esta roto y no se importa: {e}", path.display());
            Json::Broken
        }
    }
}

/// Elementos de una lista JSON que se pueden leer; los demas se cuentan aparte.
fn parse_list<T: DeserializeOwned>(v: Value, what: &str) -> (Vec<T>, usize) {
    let Value::Array(items) = v else {
        eprintln!("Importacion: {what} no es una lista");
        return (Vec::new(), 1);
    };
    let mut bad = 0;
    let out = items
        .into_iter()
        .filter_map(|x| match serde_json::from_value(x) {
            Ok(t) => Some(t),
            Err(e) => {
                bad += 1;
                eprintln!("Importacion: elemento ilegible en {what}: {e}");
                None
            }
        })
        .collect();
    (out, bad)
}

/// Pares clave-valor de un objeto JSON que se pueden leer.
fn parse_map<T: DeserializeOwned>(v: Value, what: &str) -> (Vec<(String, T)>, usize) {
    let Value::Object(map) = v else {
        eprintln!("Importacion: {what} no es un objeto");
        return (Vec::new(), 1);
    };
    let mut bad = 0;
    let out = map
        .into_iter()
        .filter_map(|(k, x)| match serde_json::from_value(x) {
            Ok(t) => Some((k, t)),
            Err(e) => {
                bad += 1;
                eprintln!("Importacion: entrada «{k}» ilegible en {what}: {e}");
                None
            }
        })
        .collect();
    (out, bad)
}

/// Resumen de un archivo para el informe: `nombre=n` / `nombre=ausente` / `nombre=roto`.
fn line(name: &str, state: &Json, n: usize, bad: usize) -> String {
    match state {
        Json::Missing => format!("{name}=ausente"),
        Json::Broken => format!("{name}=roto"),
        Json::Value(_) if bad > 0 => format!("{name}={n} ({bad} ilegibles)"),
        Json::Value(_) => format!("{name}={n}"),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Importa los JSON de `dir` si aun no se hizo. Devuelve el informe cuando ha importado.
/// Si algo falla al escribir en la base se deshace todo y no se marca como hecho (se reintentara).
pub fn import_json_once(conn: &mut Connection, dir: &Path) -> Result<Option<String>> {
    if meta_get(conn, META_IMPORTED)?.is_some() {
        return Ok(None);
    }
    let tx = conn.transaction()?;
    let report = import_all(&tx, dir)?;
    meta_set(&tx, META_IMPORTED, &now_ms().to_string())?;
    meta_set(&tx, META_REPORT, &report)?;
    tx.commit()?;
    eprintln!("Importacion de los JSON a visor.db: {report}");
    Ok(Some(report))
}

/// Importa todos los JSON conocidos con la conexion (o transaccion) dada. No marca nada en `meta`.
pub fn import_all(c: &Connection, dir: &Path) -> Result<String> {
    let mut rep: Vec<String> = Vec::new();

    // favorites.json: ["ruta", ...]
    let st = read_json(&dir.join("favorites.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (list, b): (Vec<String>, usize) = parse_list(v.clone(), "favorites.json");
        bad = b;
        for p in list.iter().filter(|p| !p.is_empty()) {
            favorite_set(c, p, true)?;
            n += 1;
        }
    }
    rep.push(line("favorites", &st, n, bad));

    // games.json: {"games":[ManualGame]}
    let st = read_json(&dir.join("games.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let games = match v {
            Value::Object(m) => m.get("games").cloned().unwrap_or(Value::Array(Vec::new())),
            _ => Value::Null,
        };
        let (list, b) = parse_list::<crate::games::ManualGame>(games, "games.json");
        bad = b;
        for g in list.iter().filter(|g| !g.id.is_empty()) {
            if manual_add(c, g)? {
                n += 1;
            }
        }
    }
    rep.push(line("games", &st, n, bad));

    // stats.json: {"<id>": {lastPlayed, seconds, sessions?}}
    let st = read_json(&dir.join("stats.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (map, b) = parse_map::<PlayStat>(v.clone(), "stats.json");
        bad = b;
        for (id, s) in map.iter().filter(|(id, _)| !id.is_empty()) {
            stat_import(c, id, s)?;
            n += 1;
        }
    }
    rep.push(line("stats", &st, n, bad));

    // hidden.json: {"hidden":[{id,name,platform}]}
    let st = read_json(&dir.join("hidden.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let list = v.get("hidden").cloned().unwrap_or(Value::Array(Vec::new()));
        let (list, b) = parse_list::<HiddenEntry>(list, "hidden.json");
        bad = b;
        for e in list.iter().filter(|e| !e.id.is_empty()) {
            hidden_set(c, e, true)?;
            n += 1;
        }
    }
    rep.push(line("hidden", &st, n, bad));

    // titleprefs.json: {"<id>": {accent?, sort?, magpie?}}
    let st = read_json(&dir.join("titleprefs.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (map, b) = parse_map::<TitlePref>(v.clone(), "titleprefs.json");
        bad = b;
        for (id, p) in map.iter().filter(|(id, _)| !id.is_empty()) {
            pref_save(c, id, Some(p))?;
            n += 1;
        }
    }
    rep.push(line("titleprefs", &st, n, bad));

    // sagas.json: {"sagas":[Saga]}
    let st = read_json(&dir.join("sagas.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let list = v.get("sagas").cloned().unwrap_or(Value::Array(Vec::new()));
        let (list, b) = parse_list::<Saga>(list, "sagas.json");
        bad = b;
        let clean = sagas::sanitize(&list);
        n = clean.len();
        sagas_replace(c, &clean)?;
    }
    rep.push(line("sagas", &st, n, bad));

    // sgdb.json: {"<titulo>": {gameId?, gameName, autoDone}}
    let st = read_json(&dir.join("sgdb.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (map, b) = parse_map::<SgdbEntry>(v.clone(), "sgdb.json");
        bad = b;
        for (id, e) in map.iter().filter(|(id, _)| !id.is_empty()) {
            sgdb_put(c, id, e)?;
            n += 1;
        }
    }
    rep.push(line("sgdb", &st, n, bad));

    // novels.json: {"<juego>": NovelPrefs}
    let st = read_json(&dir.join("novels.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (map, b) = parse_map::<NovelPrefs>(v.clone(), "novels.json");
        bad = b;
        for (id, p) in map.iter().filter(|(id, _)| !id.is_empty()) {
            novel_prefs_put(c, id, p)?;
            n += 1;
        }
    }
    rep.push(line("novels", &st, n, bad));

    // known-words.json: ["palabra", ...]
    let st = read_json(&dir.join("known-words.json"));
    let (mut n, mut bad) = (0, 0);
    if let Json::Value(v) = &st {
        let (list, b): (Vec<String>, usize) = parse_list(v.clone(), "known-words.json");
        bad = b;
        for w in list.iter().filter(|w| !w.trim().is_empty()) {
            known_set(c, w, true)?;
            n += 1;
        }
    }
    rep.push(line("known", &st, n, bad));

    // phrases/<juego>.json: [Phrase]. La subcarpeta phrases/media (capturas y audio) no se toca.
    let (mut files, mut broken, mut n, mut bad) = (0, 0, 0, 0);
    if let Ok(rd) = fs::read_dir(dir.join("phrases")) {
        let mut paths: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().map(|x| x == "json").unwrap_or(false)).collect();
        paths.sort();
        for p in paths {
            files += 1;
            match read_json(&p) {
                Json::Value(v) => {
                    let (list, b) = parse_list::<Phrase>(v, &p.display().to_string());
                    bad += b;
                    for ph in list.iter().filter(|ph| !ph.text.is_empty()) {
                        phrase_save(c, ph)?;
                        n += 1;
                    }
                }
                _ => broken += 1,
            }
        }
    }
    rep.push(format!("phrases={n} en {files} archivos ({broken} rotos, {bad} elementos ilegibles)"));

    Ok(rep.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, DB_FILE};

    fn w(d: &Path, name: &str, text: &str) {
        let p = d.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn all_json(d: &Path) {
        w(d, "favorites.json", r#"["D:\\Caps\\a.png","D:\\Caps\\b.png"]"#);
        w(d, "games.json", r#"{"games":[{"id":"manual:1","name":"Mi juego","kind":"exe","exe":"a.exe","args":"-w","platform":"pc"},{"id":"manual:2","name":"Zelda","kind":"emulator","exe":"e.exe","args":"-g {rom}","rom":"z.nsp","platform":"switch","folder":"Zelda TOTK"}]}"#);
        w(d, "stats.json", r#"{"manual:1":{"lastPlayed":5000,"seconds":120,"sessions":[{"start":1000,"end":61000},{"start":100000,"end":160000}]},"steam:7":{"lastPlayed":9,"seconds":7200}}"#);
        w(d, "hidden.json", r#"{"hidden":[{"id":"pc/x","name":"X","platform":"pc"}]}"#);
        w(d, "titleprefs.json", r##"{"manual:1":{"accent":"#00a0d8","sort":"name","magpie":{"mode":"2x","native":[1280,720],"monitor":[2560,1440],"class":"UnityWndClass","exe":"juego.exe","summary":"1280x720 a 2x","managed":true}},"pc/x":{"sort":"favs"}}"##);
        w(d, "sagas.json", r#"{"sagas":[{"id":"s1","name":"Mass Effect","members":["steam:1","steam:2"]},{"id":"s2","name":"Zelda","members":["pc/zelda"]}]}"#);
        w(d, "sgdb.json", r#"{"manual:1":{"gameId":55,"gameName":"Mi juego","autoDone":true},"pc/y":{"gameId":null,"gameName":"","autoDone":true}}"#);
        w(d, "novels.json", r#"{"manual:9":{"hookCodes":["/HS-4@0"],"reader":{"furigana":"off","posColors":false,"fontSize":40}}}"#);
        w(d, "known-words.json", r#"["雪","降る"]"#);
        w(d, "phrases/g1-aaaa.json", r#"[{"id":"p1","gameId":"manual:9","gameName":"Novela","text":"雪が降る","translation":"Nieva","starred":true,"tags":["a","b"],"seen":3,"review":{"due":777,"intervalDays":3.0,"ease":2.2,"reps":2,"lapses":1},"explanation":{"translation":"Nieva","chain":[{"kind":"v","piece":"雪","meaning":"nieve"}]},"context":[{"who":null,"text":"antes"}]}]"#);
        w(d, "phrases/g2-bbbb.json", r#"[{"gameId":"manual:10","text":"駅まで行く"}]"#);
        w(d, "phrases/media/img.png", "no es json");
    }

    #[test]
    fn imports_every_json_and_keeps_originals() {
        let d = tempfile::tempdir().unwrap();
        all_json(d.path());
        let before: Vec<_> = ["favorites.json", "games.json", "stats.json", "novels.json", "phrases/g1-aaaa.json"].iter().map(|f| fs::read(d.path().join(f)).unwrap()).collect();
        let db = Db::open(d.path()).unwrap();
        db.with(|c| {
            assert_eq!(favorites_load(c).unwrap(), vec!["D:\\Caps\\a.png", "D:\\Caps\\b.png"]);
            let m = manual_load(c).unwrap();
            assert_eq!(m.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(), ["manual:1", "manual:2"]);
            assert_eq!((m[0].folder.clone(), m[1].folder.clone(), m[1].rom.as_str()), (None, Some("Zelda TOTK".to_string()), "z.nsp"));
            let st = stats_load(c).unwrap();
            assert_eq!((st["manual:1"].last_played, st["manual:1"].seconds, st["manual:1"].sessions.len()), (5000, 120, 2));
            assert_eq!((st["steam:7"].seconds, st["steam:7"].sessions.len()), (7200, 0));
            assert_eq!(hidden_load(c).unwrap().len(), 1);
            let p = prefs_load(c).unwrap();
            let mg = p["manual:1"].magpie.clone().unwrap();
            assert_eq!((p["manual:1"].accent.as_deref(), p["manual:1"].sort.as_deref()), (Some("#00a0d8"), Some("name")));
            assert_eq!((mg.mode.as_str(), mg.native, mg.monitor, mg.managed, mg.class.as_deref()), ("2x", Some((1280, 720)), Some((2560, 1440)), true, Some("UnityWndClass")));
            assert!(p["pc/x"].magpie.is_none());
            let s = sagas_load(c).unwrap();
            assert_eq!((s.len(), s[0].name.as_str(), s[0].members.clone()), (2, "Mass Effect", vec!["steam:1".to_string(), "steam:2".to_string()]));
            let g = sgdb_load(c).unwrap();
            assert_eq!((g["manual:1"].game_id, g["pc/y"].game_id, g["pc/y"].auto_done), (Some(55), None, true));
            let n = novel_prefs_get(c, "manual:9").unwrap().unwrap();
            assert_eq!((n.hook_codes.clone(), n.reader.font_size, n.filters.dedupe_chars), (vec!["/HS-4@0".to_string()], 40, true));
            assert_eq!(known_load(c).unwrap().len(), 2);
            let ph = phrase_get(c, "manual:9", "p1").unwrap().unwrap();
            assert_eq!((ph.translation.as_deref(), ph.starred, ph.tags.clone(), ph.seen, ph.review.due, ph.review.reps), (Some("Nieva"), true, vec!["a".to_string(), "b".to_string()], 3, 777, 2));
            assert_eq!(ph.explanation.unwrap().chain[0].meaning, "nieve");
            assert_eq!(ph.context[0].text, "antes");
            // La frase sin id recibe el id estable.
            assert_eq!(phrases_load_game(c, "manual:10").unwrap()[0].id, crate::jp::phrases::phrase_id("manual:10", "駅まで行く"));
            assert_eq!(phrases_load_all(c).unwrap().len(), 2);
            let rep = meta_get(c, META_REPORT).unwrap().unwrap();
            assert!(rep.contains("favorites=2") && rep.contains("phrases=2 en 2 archivos"), "{rep}");
        });
        // Los JSON siguen igual.
        let after: Vec<_> = ["favorites.json", "games.json", "stats.json", "novels.json", "phrases/g1-aaaa.json"].iter().map(|f| fs::read(d.path().join(f)).unwrap()).collect();
        assert_eq!(before, after);
        assert!(d.path().join("phrases/media/img.png").exists());
    }

    #[test]
    fn import_happens_only_once() {
        let d = tempfile::tempdir().unwrap();
        w(d.path(), "favorites.json", r#"["a.png"]"#);
        {
            let db = Db::open(d.path()).unwrap();
            db.with(|c| {
                assert!(meta_get(c, META_IMPORTED).unwrap().is_some());
                favorite_set(c, "a.png", false).unwrap(); // el usuario la quita
            });
        }
        // Al reabrir no se vuelve a importar (la favorita no reaparece) aunque el JSON siga ahi.
        let db = Db::open(d.path()).unwrap();
        assert!(db.with(favorites_load).unwrap().is_empty());
        assert!(d.path().join(DB_FILE).exists() && d.path().join("favorites.json").exists());
    }

    #[test]
    fn empty_dir_creates_empty_db_and_marks_imported() {
        let d = tempfile::tempdir().unwrap();
        let db = Db::open(&d.path().join("nueva")).unwrap();
        db.with(|c| {
            assert!(favorites_load(c).unwrap().is_empty() && stats_load(c).unwrap().is_empty());
            assert!(meta_get(c, META_REPORT).unwrap().unwrap().contains("favorites=ausente"));
        });
    }

    #[test]
    fn broken_json_is_skipped_and_the_rest_survives() {
        let d = tempfile::tempdir().unwrap();
        all_json(d.path());
        w(d.path(), "stats.json", "{ esto esta roto");
        w(d.path(), "known-words.json", r#"["雪", 5, "降る"]"#); // un elemento malo no tira los demas
        w(d.path(), "phrases/g1-aaaa.json", "[ roto");
        let db = Db::open(d.path()).unwrap();
        db.with(|c| {
            assert!(stats_load(c).unwrap().is_empty());
            assert_eq!(known_load(c).unwrap().len(), 2);
            assert_eq!(favorites_load(c).unwrap().len(), 2);
            assert_eq!(manual_load(c).unwrap().len(), 2);
            assert_eq!(sagas_load(c).unwrap().len(), 2);
            // Del glosario solo se pierde el archivo roto.
            assert_eq!(phrases_load_all(c).unwrap().len(), 1);
            let rep = meta_get(c, META_REPORT).unwrap().unwrap();
            assert!(rep.contains("stats=roto") && rep.contains("known=2 (1 ilegibles)") && rep.contains("1 rotos"), "{rep}");
        });
        assert_eq!(fs::read_to_string(d.path().join("stats.json")).unwrap(), "{ esto esta roto");
    }

    #[test]
    fn unreadable_entries_inside_a_file_are_skipped() {
        let d = tempfile::tempdir().unwrap();
        w(d.path(), "stats.json", r#"{"a":{"lastPlayed":1,"seconds":5},"b":"mal","c":{"lastPlayed":2,"seconds":6}}"#);
        w(d.path(), "games.json", r#"{"games":[{"id":"manual:1","name":"A","kind":"exe","platform":"pc"},{"nombre":"sin id"}]}"#);
        let db = Db::open(d.path()).unwrap();
        db.with(|c| {
            assert_eq!(stats_load(c).unwrap().len(), 2);
            assert_eq!(manual_load(c).unwrap().len(), 1);
        });
    }
}

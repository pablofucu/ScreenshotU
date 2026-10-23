//! Base de datos SQLite unica del visor (`<config>/visor.db`): favoritas, juegos anadidos a mano,
//! estadisticas y partidas, juegos ocultos, preferencias por juego, sagas, SteamGridDB, novelas
//! japonesas (preferencias, palabras conocidas y glosario de frases).
//!
//! * El esquema esta versionado con `PRAGMA user_version` y una lista de migraciones
//!   ([`MIGRATIONS`]): para anadir tablas basta con **anadir al final** una migracion nueva (nunca
//!   se edita una ya publicada).
//! * La primera vez se importan los JSON antiguos (ver [`import`]); los JSON no se tocan.
//! * El acceso concreto a cada tipo de dato esta en [`data`].
//! * Para la copia de seguridad: [`Db::backup_to`] y [`validate_file`].
use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub mod data;
pub mod import;

/// Nombre del archivo dentro de la carpeta de configuracion.
pub const DB_FILE: &str = "visor.db";

/// `PRAGMA application_id` de las bases del visor (los bytes de "VISR"): permite reconocer un archivo ajeno.
/// Debe coincidir con el valor que fija la migracion 1.
pub const APPLICATION_ID: i64 = 0x5649_5352;

/// Migraciones en orden. La migracion `i` (0-based) lleva la base de la version `i` a la `i + 1`.
/// **Solo se anade al final**; `user_version` guarda cuantas se han aplicado.
pub const MIGRATIONS: &[&str] = &[
    // 1: esquema inicial.
    r#"
    CREATE TABLE meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    -- Capturas favoritas (ruta absoluta). El orden es el de insercion (rowid).
    CREATE TABLE favorites (
        path TEXT PRIMARY KEY
    );

    -- Juegos anadidos a mano (antes games.json). El orden es el de insercion (rowid).
    CREATE TABLE games (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        kind     TEXT NOT NULL,
        exe      TEXT NOT NULL DEFAULT '',
        args     TEXT NOT NULL DEFAULT '',
        rom      TEXT NOT NULL DEFAULT '',
        platform TEXT NOT NULL,
        folder   TEXT
    );

    -- Totales por juego y partidas (antes stats.json). `seconds` incluye el tiempo anterior a guardar partidas.
    CREATE TABLE play_stats (
        game_id        TEXT PRIMARY KEY,
        last_played_ms INTEGER NOT NULL DEFAULT 0,
        seconds        INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE sessions (
        id       INTEGER PRIMARY KEY,
        game_id  TEXT NOT NULL REFERENCES play_stats(game_id) ON DELETE CASCADE ON UPDATE CASCADE,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL
    );
    CREATE INDEX sessions_game  ON sessions(game_id, start_ms);
    CREATE INDEX sessions_start ON sessions(start_ms);

    -- Juegos ocultos (antes hidden.json). El orden es el de insercion (rowid).
    CREATE TABLE hidden (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        platform TEXT NOT NULL
    );

    -- Preferencias por titulo (antes titleprefs.json) y su escalado con Magpie.
    CREATE TABLE title_prefs (
        title_id TEXT PRIMARY KEY,
        accent   TEXT,
        sort     TEXT
    );
    CREATE TABLE title_magpie (
        title_id  TEXT PRIMARY KEY REFERENCES title_prefs(title_id) ON DELETE CASCADE ON UPDATE CASCADE,
        mode      TEXT NOT NULL,
        native_w  INTEGER,
        native_h  INTEGER,
        monitor_w INTEGER,
        monitor_h INTEGER,
        class     TEXT,
        exe       TEXT,
        summary   TEXT,
        managed   INTEGER NOT NULL DEFAULT 0
    );

    -- Sagas (antes sagas.json).
    CREATE TABLE sagas (
        id       TEXT PRIMARY KEY,
        name     TEXT NOT NULL,
        position INTEGER NOT NULL
    );
    CREATE TABLE saga_members (
        saga_id  TEXT NOT NULL REFERENCES sagas(id) ON DELETE CASCADE ON UPDATE CASCADE,
        title_id TEXT NOT NULL,
        position INTEGER NOT NULL,
        PRIMARY KEY (saga_id, title_id)
    );
    CREATE INDEX saga_members_title ON saga_members(title_id);

    -- Juego de SteamGridDB de cada titulo (antes sgdb.json).
    CREATE TABLE sgdb (
        title_id  TEXT PRIMARY KEY,
        game_id   INTEGER,
        game_name TEXT NOT NULL DEFAULT '',
        auto_done INTEGER NOT NULL DEFAULT 0
    );

    -- Novelas japonesas: preferencias por juego (bloque anidado, se guarda como JSON; antes novels.json).
    CREATE TABLE novel_prefs (
        game_id    TEXT PRIMARY KEY,
        prefs_json TEXT NOT NULL
    );
    -- Palabras conocidas, globales (antes known-words.json).
    CREATE TABLE known_words (
        word TEXT PRIMARY KEY
    );

    -- Glosario de frases (antes phrases/<juego>.json). Lo anidado (tokens, palabras, gramatica,
    -- contexto, explicacion) va en columnas JSON; lo que se filtra u ordena, en columnas propias.
    CREATE TABLE phrases (
        game_id          TEXT NOT NULL,
        id               TEXT NOT NULL,
        game_name        TEXT NOT NULL DEFAULT '',
        who              TEXT,
        text             TEXT NOT NULL,
        toks_json        TEXT NOT NULL DEFAULT '[]',
        words_json       TEXT NOT NULL DEFAULT '[]',
        grammar_json     TEXT NOT NULL DEFAULT '[]',
        context_json     TEXT NOT NULL DEFAULT '[]',
        created_at       INTEGER NOT NULL DEFAULT 0,
        last_seen_at     INTEGER NOT NULL DEFAULT 0,
        seen             INTEGER NOT NULL DEFAULT 1,
        translation      TEXT,
        explanation_json TEXT,
        starred          INTEGER NOT NULL DEFAULT 0,
        note             TEXT NOT NULL DEFAULT '',
        shot             TEXT,
        audio            TEXT,
        synth            TEXT,
        anki_note_id     INTEGER,
        due_ms           INTEGER NOT NULL DEFAULT 0,
        interval_days    REAL NOT NULL DEFAULT 0,
        ease             REAL NOT NULL DEFAULT 2.5,
        reps             INTEGER NOT NULL DEFAULT 0,
        lapses           INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (game_id, id)
    );
    CREATE INDEX phrases_due  ON phrases(due_ms);
    CREATE INDEX phrases_seen ON phrases(last_seen_at);
    CREATE TABLE phrase_tags (
        game_id   TEXT NOT NULL,
        phrase_id TEXT NOT NULL,
        position  INTEGER NOT NULL,
        tag       TEXT NOT NULL,
        PRIMARY KEY (game_id, phrase_id, position),
        FOREIGN KEY (game_id, phrase_id) REFERENCES phrases(game_id, id) ON DELETE CASCADE ON UPDATE CASCADE
    );

    PRAGMA application_id = 1447646034;
    "#,
    // 2: estado de cada titulo (pendiente, jugando, terminado, abandonado) y fecha de fin.
    r#"
    CREATE TABLE title_status (
        title_id    TEXT PRIMARY KEY,
        status      TEXT NOT NULL,
        finished_ms INTEGER,
        updated_ms  INTEGER NOT NULL DEFAULT 0
    );
    "#,
    // 3: estadisticas de lectura de las novelas (sesiones, totales por dia, fecha de palabras conocidas).
    crate::jp::readstats::SCHEMA,
    // 4: catalogo de capturas con su huella (base de la sincronizacion con la nube).
    crate::catalog::SCHEMA,
    // 5: rutas de partidas guardadas (manuales y detectadas desactivadas).
    crate::saves::SCHEMA,
    // 6: tipos de grafismo que SteamGridDB ya no tiene para un titulo (no se vuelven a contar como pendientes).
    r#"
    ALTER TABLE sgdb ADD COLUMN tried TEXT NOT NULL DEFAULT '';
    "#,
    // 7: «Mi lista»: nota (1-5) de los terminados, orden manual de los pendientes y «dejarlo» de los juegos parados.
    r#"
    ALTER TABLE title_status ADD COLUMN rating INTEGER;
    ALTER TABLE title_status ADD COLUMN list_pos INTEGER;
    ALTER TABLE title_status ADD COLUMN snooze_ms INTEGER;
    "#,
    // 8: colecciones (antes sagas): categoria a la que pertenecen y color manual. El orden entre colecciones
    // es `sagas.position`. La categoria de las sagas antiguas se rellena al cargar la biblioteca (`sagas::convert_legacy`).
    r#"
    ALTER TABLE sagas ADD COLUMN platform TEXT;
    ALTER TABLE sagas ADD COLUMN color INTEGER;
    "#,
    // 9: fecha de lanzamiento de cada titulo (mismo id que el resto de datos por titulo). Año siempre; mes y dia
    // opcionales. `auto` = sacada de SteamGridDB (se puede sustituir sola); 0 = puesta a mano (no se pisa nunca).
    r#"
    CREATE TABLE title_release (
        title_id TEXT PRIMARY KEY,
        year     INTEGER NOT NULL,
        month    INTEGER,
        day      INTEGER,
        auto     INTEGER NOT NULL DEFAULT 0
    );
    "#,
];

/// Version del esquema que entiende esta compilacion.
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

/// Aplica las migraciones pendientes (cada una en su transaccion, subiendo `user_version`).
/// Si la base es de una version mas nueva que la del programa, falla sin tocarla.
pub fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > migrations.len() as i64 {
        bail!("la base de datos es de una version mas nueva ({current}) que la que entiende este programa ({})", migrations.len());
    }
    for (i, sql) in migrations.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql).with_context(|| format!("migracion {} de la base de datos", i + 1))?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}

fn prepare(conn: &Connection) -> Result<()> {
    // `journal_mode` devuelve una fila: no vale `execute`.
    let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

/// Abre (creando si hace falta) una base con sus pragmas y el esquema al dia, sin importar nada.
pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    }
    let mut conn = Connection::open(path).with_context(|| format!("abriendo {}", path.display()))?;
    prepare(&conn)?;
    pre_migration_copy(&conn, path);
    migrate(&mut conn, MIGRATIONS)?;
    Ok(conn)
}

/// Antes de subir una base ya existente a un esquema nuevo (p. ej. al actualizar el programa) guarda una
/// copia `visor.db.antes-vN.bak` (N = version previa). Si falla no se impide migrar: solo se avisa.
fn pre_migration_copy(conn: &Connection, path: &Path) {
    let Ok(current) = conn.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0)) else { return };
    if current < 1 || current >= latest_version() {
        return;
    }
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(format!(".antes-v{current}.bak"));
    if let Err(e) = backup_to(conn, &path.with_file_name(name)) {
        eprintln!("copia previa a la migracion: {e:#}");
    }
}

/// Conexion compartida: un `Mutex<Connection>` (SQLite serializa las escrituras de todos modos).
/// Los hilos del lanzador, la captura, el glosario y las novelas la usan a traves de [`Db::with`].
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// `<config_dir>/visor.db`: la abre y, si aun no se importaron los JSON antiguos, los importa.
    pub fn open(config_dir: &Path) -> Result<Db> {
        let mut conn = open_connection(&config_dir.join(DB_FILE))?;
        import::import_json_once(&mut conn, config_dir)?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    /// Una base en memoria con el esquema (pruebas, o respaldo si el disco falla).
    pub fn open_memory() -> Result<Db> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    /// Una base en un archivo concreto, sin importar JSON.
    pub fn open_path(path: &Path) -> Result<Db> {
        Ok(Db { conn: Mutex::new(open_connection(path)?) })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Ejecuta `f` con la conexion (bloqueada mientras dure). No llames a `with` dentro de `f`.
    pub fn with<R>(&self, f: impl FnOnce(&Connection) -> R) -> R {
        f(&self.lock())
    }

    /// Copia consistente de la base viva a `dest` (ver [`backup_to`]).
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        backup_to(&self.lock(), dest)
    }
}

/// Copia consistente de la base a otro archivo con `VACUUM INTO` (una sola instantanea, sin `-wal`
/// ni `-shm`, compactada). Escribe en un temporal junto al destino y lo renombra, asi que un destino
/// existente se sustituye entero o no se toca.
pub fn backup_to(conn: &Connection, dest: &Path) -> Result<()> {
    if let Some(dir) = dest.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creando {}", dir.display()))?;
    }
    let mut tmp_name = dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    tmp_name.push(".part");
    let tmp: PathBuf = dest.with_file_name(tmp_name);
    let _ = std::fs::remove_file(&tmp);
    let r = conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()]).context("copiando la base de datos");
    if let Err(e) = r {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, dest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| format!("escribiendo {}", dest.display()));
    }
    Ok(())
}

/// Lo que se sabe de un archivo validado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbInfo {
    /// `user_version` del archivo.
    pub version: i64,
    /// Tamano en bytes.
    pub size: u64,
}

/// Comprueba que `path` es una base del visor que este programa puede usar: es SQLite, es nuestra
/// (`application_id`), pasa la comprobacion de integridad, tiene las tablas base y su version no es
/// mas nueva que la de este programa. Se abre en solo lectura y no se modifica.
pub fn validate_file(path: &Path) -> Result<DbInfo> {
    let meta = std::fs::metadata(path).with_context(|| format!("no se puede leer {}", path.display()))?;
    if !meta.is_file() {
        bail!("{} no es un archivo", path.display());
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .context("no es una base de datos SQLite")?;
    let app_id: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0)).context("no es una base de datos SQLite")?;
    if app_id != APPLICATION_ID {
        bail!("no es una base de datos del visor");
    }
    let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0)).context("no es una base de datos SQLite")?;
    if check != "ok" {
        bail!("la base de datos esta danada: {check}");
    }
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        bail!("la base de datos esta vacia");
    }
    if version > latest_version() {
        bail!("la base de datos es de una version mas nueva ({version}) que la que entiende este programa ({})", latest_version());
    }
    for t in ["meta", "favorites", "play_stats", "sessions"] {
        let n: i64 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [t], |r| r.get(0))?;
        if n == 0 {
            bail!("a la base de datos le falta la tabla {t}");
        }
    }
    Ok(DbInfo { version, size: meta.len() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use data::*;

    fn tables(c: &Connection) -> Vec<String> {
        let mut st = c.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap();
        st.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
    }

    #[test]
    fn schema_created_with_pragmas() {
        let d = tempfile::tempdir().unwrap();
        let c = open_connection(&d.path().join("sub/visor.db")).unwrap();
        let t = tables(&c);
        for want in ["favorites", "games", "play_stats", "sessions", "hidden", "title_prefs", "title_magpie", "sagas", "saga_members", "sgdb", "novel_prefs", "known_words", "phrases", "phrase_tags", "title_status", "save_paths", "meta"] {
            assert!(t.iter().any(|x| x == want), "falta la tabla {want}: {t:?}");
        }
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, latest_version());
        let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
        let app: i64 = c.query_row("PRAGMA application_id", [], |r| r.get(0)).unwrap();
        assert_eq!(app, APPLICATION_ID);
    }

    #[test]
    fn migrations_are_incremental_and_idempotent() {
        let mut c = Connection::open_in_memory().unwrap();
        let m1: &[&str] = &["CREATE TABLE a (x INTEGER);"];
        migrate(&mut c, m1).unwrap();
        c.execute("INSERT INTO a VALUES (1)", []).unwrap();
        migrate(&mut c, m1).unwrap(); // no repite
        let m2: &[&str] = &["CREATE TABLE a (x INTEGER);", "CREATE TABLE b (y TEXT); INSERT INTO a VALUES (2);"];
        migrate(&mut c, m2).unwrap();
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        let n: i64 = c.query_row("SELECT count(*) FROM a", [], |r| r.get(0)).unwrap();
        assert_eq!((v, n), (2, 2), "la migracion 1 no se repite y la 2 se aplica una vez");
        // Una migracion que falla no deja nada a medias ni sube la version.
        let m3: &[&str] = &[m2[0], m2[1], "CREATE TABLE c (z); SELECT * FROM no_existe;"];
        assert!(migrate(&mut c, m3).is_err());
        assert_eq!(c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0)).unwrap(), 2);
        assert!(!tables(&c).contains(&"c".to_string()));
        // Una base mas nueva que el programa se rechaza.
        assert!(migrate(&mut c, m1).is_err());
    }

    #[test]
    fn reopen_keeps_data_and_version() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let db = Db::open_path(&p).unwrap();
            db.with(|c| favorite_set(c, "a.png", true)).unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        assert_eq!(db.with(favorites_load).unwrap(), vec!["a.png"]);
    }

    #[test]
    fn backup_is_consistent_and_validates() {
        let d = tempfile::tempdir().unwrap();
        let db = Db::open_path(&d.path().join("visor.db")).unwrap();
        db.with(|c| {
            favorite_set(c, "x.png", true).unwrap();
            stat_add_session(c, "manual:1", 1000, 61_000).unwrap();
        });
        let dest = d.path().join("copias/visor-copia.db");
        db.backup_to(&dest).unwrap();
        let info = validate_file(&dest).unwrap();
        assert_eq!(info.version, latest_version());
        assert!(info.size > 0);
        // Sobrescribir un destino existente tambien funciona y no deja temporales.
        db.with(|c| favorite_set(c, "y.png", true)).unwrap();
        db.backup_to(&dest).unwrap();
        assert!(!d.path().join("copias/visor-copia.db.part").exists());
        let copy = Db::open_path(&dest).unwrap();
        assert_eq!(copy.with(favorites_load).unwrap(), vec!["x.png", "y.png"]);
        assert_eq!(copy.with(stats_load).unwrap()["manual:1"].seconds, 60);
    }

    #[test]
    fn validate_rejects_foreign_and_broken_files() {
        let d = tempfile::tempdir().unwrap();
        assert!(validate_file(&d.path().join("no_existe.db")).is_err());
        let txt = d.path().join("a.db");
        std::fs::write(&txt, "esto no es sqlite, ni de lejos, pero tiene mas de cien bytes de texto para que SQLite lo mire de verdad y lo rechace sin mas.........").unwrap();
        assert!(validate_file(&txt).is_err());
        // SQLite valida pero ajena.
        let other = d.path().join("otra.db");
        Connection::open(&other).unwrap().execute_batch("CREATE TABLE t (a); PRAGMA user_version = 1;").unwrap();
        assert!(validate_file(&other).unwrap_err().to_string().contains("no es una base de datos del visor"));
        // Del visor, pero de una version futura.
        let fut = d.path().join("futura.db");
        {
            let db = Db::open_path(&fut).unwrap();
            db.with(|c| c.execute_batch(&format!("PRAGMA user_version = {}", latest_version() + 1))).unwrap();
        }
        assert!(validate_file(&fut).unwrap_err().to_string().contains("mas nueva"));
    }

    #[test]
    fn upgrading_an_existing_base_leaves_a_copy_of_the_old_version() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let mut c = Connection::open(&p).unwrap();
            migrate(&mut c, &MIGRATIONS[..1]).unwrap();
            c.execute("INSERT INTO favorites(path) VALUES ('a.png')", []).unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        let bak = d.path().join("visor.db.antes-v1.bak");
        assert!(bak.exists());
        // La copia es la base antigua (version 1) y conserva los datos.
        let old = Connection::open(&bak).unwrap();
        assert_eq!(old.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0)).unwrap(), 1);
        assert_eq!(old.query_row::<i64, _, _>("SELECT count(*) FROM favorites", [], |r| r.get(0)).unwrap(), 1);
        drop(db);
        // Una base nueva o ya al dia no deja copias.
        let q = d.path().join("nueva.db");
        drop(Db::open_path(&q).unwrap());
        drop(Db::open_path(&q).unwrap());
        assert!(!d.path().join("nueva.db.antes-v0.bak").exists());
        assert!(!d.path().join(format!("nueva.db.antes-v{}.bak", latest_version())).exists());
    }

    #[test]
    fn status_persists_and_migration_2_upgrades_a_v1_base() {
        // Una base en version 1 (sin title_status) sube sola a la 2 sin perder datos.
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let mut c = Connection::open(&p).unwrap();
            migrate(&mut c, &MIGRATIONS[..1]).unwrap();
            c.execute("INSERT INTO favorites(path) VALUES ('a.png')", []).unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        assert_eq!(db.with(favorites_load).unwrap(), vec!["a.png"]);
        assert_eq!(db.with(|c| c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))).unwrap(), latest_version());
        db.with(|c| {
            status_save(c, "steam:1", Some(("finished", Some(5000))), 6000).unwrap();
            status_save(c, "steam:2", Some(("playing", None)), 6000).unwrap();
            pref_save(c, "steam:2", Some(&crate::userdata::TitlePref { sort: Some("name".into()), ..Default::default() })).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:1"].status.as_deref(), prefs["steam:1"].finished_ms), (Some("finished"), Some(5000)));
        assert_eq!((prefs["steam:2"].status.as_deref(), prefs["steam:2"].sort.as_deref()), (Some("playing"), Some("name")));
        db.with(|c| status_save(c, "steam:1", None, 7000)).unwrap();
        assert!(!db.with(prefs_load).unwrap().contains_key("steam:1"));
    }

    #[test]
    fn migration_7_adds_list_columns_and_keeps_old_status() {
        // Una base en version 6 con un estado sube a la 7 sin perderlo; nota, orden y «dejarlo» se guardan y se leen.
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let mut c = Connection::open(&p).unwrap();
            migrate(&mut c, &MIGRATIONS[..6]).unwrap();
            c.execute("INSERT INTO title_status(title_id, status, finished_ms, updated_ms) VALUES ('steam:1', 'finished', 5000, 6000)", []).unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        assert_eq!(db.with(|c| c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))).unwrap(), latest_version());
        let prefs = db.with(prefs_load).unwrap();
        let old = &prefs["steam:1"];
        assert_eq!((old.status.as_deref(), old.finished_ms, old.rating, old.list_pos, old.snooze_ms, old.status_ms), (Some("finished"), Some(5000), None, None, None, Some(6000)));
        db.with(|c| {
            status_save(c, "steam:1", Some(("finished", Some(5000))), 7000).unwrap();
            status_extra_save(c, "steam:1", Some(4), None, None).unwrap();
            status_save(c, "steam:2", Some(("pending", None)), 7000).unwrap();
            status_save(c, "steam:3", Some(("pending", None)), 7000).unwrap();
            status_extra_save(c, "steam:2", None, Some(2), None).unwrap();
            status_extra_save(c, "steam:3", None, Some(1), None).unwrap();
            status_save(c, "steam:4", Some(("playing", None)), 7000).unwrap();
            status_extra_save(c, "steam:4", None, None, Some(8000)).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:1"].rating, prefs["steam:1"].status_ms), (Some(4), Some(7000)));
        assert_eq!((prefs["steam:2"].list_pos, prefs["steam:3"].list_pos), (Some(2), Some(1)));
        assert_eq!(prefs["steam:4"].snooze_ms, Some(8000));
        // Cambiar el estado con status_save no pisa las columnas nuevas (las fija quien llama con status_extra_save).
        db.with(|c| {
            status_save(c, "steam:2", Some(("playing", None)), 9000).unwrap();
            status_extra_save(c, "steam:2", None, None, None).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:2"].status.as_deref(), prefs["steam:2"].list_pos), (Some("playing"), None));
        // Quitar el estado borra tambien nota y orden.
        db.with(|c| status_save(c, "steam:1", None, 9500)).unwrap();
        assert!(!db.with(prefs_load).unwrap().contains_key("steam:1"));
    }

    #[test]
    fn migration_8_adds_collection_columns_and_converts_old_sagas() {
        // Una base en version 7 con sagas sube a la 8 sin perderlas; luego se convierten con la biblioteca cargada.
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let mut c = Connection::open(&p).unwrap();
            migrate(&mut c, &MIGRATIONS[..7]).unwrap();
            c.execute_batch(
                "INSERT INTO sagas(id, name, position) VALUES ('s1', 'Mezcla', 0), ('s2', 'Perdida', 1);
                 INSERT INTO saga_members(saga_id, title_id, position) VALUES ('s1', 'pc/a', 0), ('s1', 'sw/b', 1), ('s1', 'pc/c', 2), ('s1', 'fuera', 3), ('s2', 'nada', 0);",
            )
            .unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        assert_eq!(db.with(|c| c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))).unwrap(), latest_version());
        let antes = db.with(sagas_load).unwrap();
        assert_eq!(antes.len(), 2);
        assert!(antes.iter().all(|s| s.platform.is_none() && s.color.is_none()));
        // Sin biblioteca cargada no se convierte nada.
        let vacio = std::collections::HashMap::new();
        assert!(db.with(|c| sagas_convert_legacy(c, &vacio)).unwrap().iter().all(|s| s.platform.is_none()));
        let map: std::collections::HashMap<String, String> =
            [("pc/a", "pc"), ("sw/b", "switch"), ("pc/c", "pc")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let out = db.with(|c| sagas_convert_legacy(c, &map)).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!((out[0].id.as_str(), out[0].platform.as_deref(), out[0].members.clone()), ("s1", Some("pc"), vec!["pc/a".to_string(), "pc/c".into(), "fuera".into()]));
        assert_eq!((out[1].platform.as_deref(), out[1].name.as_str(), out[1].members.clone()), (Some("switch"), "Mezcla", vec!["sw/b".to_string()]));
        assert_eq!(out[2].platform, None, "la que no tiene ningun juego conocido espera");
        // Persistido: al recargar sigue igual y una segunda pasada no cambia nada.
        assert_eq!(db.with(sagas_load).unwrap(), out);
        assert_eq!(db.with(|c| sagas_convert_legacy(c, &map)).unwrap(), out);
    }

    #[test]
    fn migration_9_adds_release_dates_and_keeps_old_prefs() {
        // Una base en version 8 con estado y acento sube a la 9 sin perderlos; la fecha de lanzamiento se guarda,
        // se lee junto al resto de datos del titulo y se quita sin tocar lo demas.
        use crate::userdata::ReleaseDate;
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("visor.db");
        {
            let mut c = Connection::open(&p).unwrap();
            migrate(&mut c, &MIGRATIONS[..8]).unwrap();
            c.execute_batch(
                "INSERT INTO title_status(title_id, status, finished_ms, updated_ms) VALUES ('steam:1', 'finished', 5000, 6000);
                 INSERT INTO title_prefs(title_id, accent, sort) VALUES ('pc/b', '#112233', NULL);",
            )
            .unwrap();
        }
        let db = Db::open_path(&p).unwrap();
        assert_eq!(db.with(|c| c.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))).unwrap(), latest_version());
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:1"].status.as_deref(), prefs["steam:1"].release), (Some("finished"), None));
        assert_eq!(prefs["pc/b"].accent.as_deref(), Some("#112233"));
        let full = ReleaseDate { year: 2017, month: Some(3), day: Some(3), auto: false };
        let auto = ReleaseDate { year: 2019, month: None, day: None, auto: true };
        db.with(|c| {
            release_save(c, "steam:1", Some(&full)).unwrap();
            release_save(c, "pc/c", Some(&auto)).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:1"].release, prefs["steam:1"].finished_ms), (Some(full), Some(5000)));
        assert_eq!(prefs["pc/c"].release, Some(auto), "un titulo solo con fecha tambien se carga");
        db.with(|c| release_save(c, "steam:1", Some(&ReleaseDate { year: 2018, ..auto }))).unwrap();
        assert_eq!(db.with(prefs_load).unwrap()["steam:1"].release.map(|r| (r.year, r.month, r.auto)), Some((2018, None, true)));
        // Quitar el estado no borra la fecha, y quitar la fecha no borra el estado.
        db.with(|c| status_save(c, "steam:1", None, 7000)).unwrap();
        assert!(db.with(prefs_load).unwrap()["steam:1"].release.is_some());
        db.with(|c| {
            status_save(c, "steam:1", Some(("playing", None)), 8000).unwrap();
            release_save(c, "steam:1", None).unwrap();
            release_save(c, "pc/c", None).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["steam:1"].status.as_deref(), prefs["steam:1"].release), (Some("playing"), None));
        assert!(!prefs.contains_key("pc/c"));
    }

    #[test]
    fn pending_order_is_saved_for_the_given_ids() {
        let db = Db::open_memory().unwrap();
        db.with(|c| {
            for id in ["a", "b", "c"] {
                status_save(c, id, Some(("pending", None)), 1).unwrap();
            }
            status_save(c, "z", Some(("playing", None)), 1).unwrap();
            pending_order_save(c, &[("c".to_string(), 1), ("a".to_string(), 2), ("b".to_string(), 3), ("z".to_string(), 9)]).unwrap();
        });
        let prefs = db.with(prefs_load).unwrap();
        assert_eq!((prefs["c"].list_pos, prefs["a"].list_pos, prefs["b"].list_pos), (Some(1), Some(2), Some(3)));
        assert_eq!(prefs["z"].list_pos, None, "solo se ordenan los pendientes");
    }

    #[test]
    fn favorites_become_portable_keeping_order() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("Capturas");
        let abs = |rel: &str| crate::relpath::to_abs(&root, rel);
        let outside = d.path().join("fuera.png").to_string_lossy().into_owned();
        let db = Db::open_memory().unwrap();
        db.with(|c| {
            favorite_set(c, &abs("PC/A/1.png"), true).unwrap();
            favorite_set(c, &outside, true).unwrap();
            favorite_set(c, "Switch/B/2.png", true).unwrap();
        });
        assert_eq!(db.with(|c| favorites_relativize(c, &root)).unwrap(), 1);
        assert_eq!(db.with(favorites_load).unwrap(), vec!["PC/A/1.png".to_string(), outside.clone(), "Switch/B/2.png".into()]);
        assert_eq!(db.with(|c| favorites_relativize(c, &root)).unwrap(), 0, "idempotente");
    }

    #[test]
    fn memory_db_works() {
        let db = Db::open_memory().unwrap();
        db.with(|c| known_set(c, "雪", true)).unwrap();
        assert_eq!(db.with(known_load).unwrap(), vec!["雪"]);
    }
}

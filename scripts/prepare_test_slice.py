import os
import shutil
import sqlite3
import json
import time

SRC_DB = r"P:\ScreenshotUL\Visor de capturas\visor.db"
SRC_CONFIG = r"P:\ScreenshotUL\Visor de capturas\config.json"
SRC_LIB = r"P:\ScreenshotUL\Visor de capturas"

BASE_DIR = r"c:\Users\pablo\Documents\Proyectos\App capturas\prueba-logros-bin"
TEST_DATA_DIR = os.path.join(BASE_DIR, "test_data")
CACHE_DIR = os.path.join(TEST_DATA_DIR, "cache")
TEST_LIB_DIR = os.path.join(BASE_DIR, "test_library")
CAPTURES_DIR = os.path.join(TEST_LIB_DIR, "Capturas")
GRAFISMOS_DIR = os.path.join(TEST_LIB_DIR, "Grafismos")
DST_DB = os.path.join(TEST_LIB_DIR, "visor.db")

SHOWCASE_GAMES = [
    {
        "id": "manual:18e09f1291ae916d",
        "name": "ELDEN RING",
        "variant": 1,
        "achievements": [
            ("ACH_ELDEN_RING", "Elden Lord", "Reached the Elden Lord ending", False, 1),
            ("ACH_AGE_OF_STARS", "Age of the Stars", "Reached the Age of the Stars ending", False, 1),
            ("ACH_MALENIA", "Shardbearer Malenia", "Defeated Shardbearer Malenia", False, 1),
            ("ACH_RADAHN", "Shardbearer Radahn", "Defeated Shardbearer Radahn", False, 1),
            ("ACH_GODRICK", "Shardbearer Godrick", "Defeated Shardbearer Godrick", False, 1),
            ("ACH_MORGOTT", "Shardbearer Morgott", "Defeated Shardbearer Morgott", False, 1),
            ("ACH_RYKARD", "Shardbearer Rykard", "Defeated Shardbearer Rykard", False, 1),
            ("ACH_MOGH", "Shardbearer Mohg", "Defeated Shardbearer Mohg", False, 1),
            ("ACH_MALIKETH", "Maliketh the Black Blade", "Defeated Maliketh the Black Blade", False, 1),
            ("ACH_HOARAH_LOUX", "Hoarah Loux the Warrior", "Defeated Hoarah Loux the Warrior", False, 1),
            ("ACH_DRAGONLORD", "Dragonlord Placidusax", "Defeated Dragonlord Placidusax", True, 0),
            ("ACH_ROUNDTABLE", "Roundtable Hold", "Arrived at Roundtable Hold", False, 1),
            ("ACH_GREAT_RUNE", "Great Rune", "Restored the power of a Great Rune", False, 1),
            ("ACH_ERDTREE_BURNING", "Erdtree Aflame", "Used kindling to set the Erdtree aflame", True, 0),
            ("ACH_LEGENDARY_ARMAMENTS", "Legendary Armaments", "Acquired all legendary armaments", False, 0),
        ]
    },
    {
        "id": "manual:18e09f12910e9163",
        "name": "Cyberpunk 2077",
        "variant": 2,
        "achievements": [
            ("ACH_THE_FOOL", "The Fool", "Become a mercenary of Night City", False, 1),
            ("ACH_THE_LOVERS", "The Lovers", "Steal the Relic with Jackie Welles", False, 1),
            ("ACH_THE_HERMIT", "The Hermit", "Find Alt Cunningham beyond the Blackwall", False, 1),
            ("ACH_THE_WHEEL", "The Wheel of Fortune", "Interrogate Anders Hellman", False, 1),
            ("ACH_THE_HIGH_PRIESTESS", "The High Priestess", "Talk with Hanako Arasaka", False, 1),
            ("ACH_THE_WORLD", "The World", "Complete the main storyline", False, 1),
            ("ACH_BREATHTAKING", "Breathtaking", "Collect all items that once belonged to Johnny Silverhand", False, 1),
            ("ACH_CITY_LIGHTS", "City Lights", "Complete all gigs and NCPD scanner hustles in City Center", False, 0),
            ("ACH_JUDY_SONG", "Judy's Song", "Complete Judy Alvarez's storyline", False, 1),
            ("ACH_PANAM_ROAD", "Life of the Road", "Complete Panam Palmer's storyline", False, 1),
            ("ACH_GUNSLINGER", "Gunslinger", "Shoot an enemy grenade mid-air with a revolver", False, 0),
            ("ACH_LEGEND_OF_AFTERLIFE", "Legend of the Afterlife", "Reach maximum Street Cred", False, 1),
        ]
    },
    {
        "id": "manual:18e09f128fae914d",
        "name": "Alan Wake 2",
        "variant": 4,
        "achievements": [
            ("ACH_ESCAPE_DARK_PLACE", "Return", "Escape the Dark Place", False, 1),
            ("ACH_WRITERS_ROOM", "The Writer's Room", "Discover Alan Wake's hidden study", False, 1),
            ("ACH_COFFEE_WORLD", "Coffee World Mascot", "Explore every attraction in Coffee World", False, 1),
            ("ACH_OVERLAP", "Cross the Overlap", "Defeat the Taken within the Overlap", False, 1),
            ("ACH_DARK_PRESENCE", "Not the Monster", "Survive the shadows without firing a weapon", True, 0),
            ("ACH_ANGELS_LAMP", "Guiding Light", "Shift reality with the Angel Lamp 50 times", False, 1),
            ("ACH_CASE_BOARD", "Mind Place Master", "Complete Saga's master case board", False, 1),
            ("ACH_HERALDS_OF_DARKNESS", "Old Gods of Asgard", "Survive the musical stage encounter", False, 1),
            ("ACH_HUNTING_RIFLE", "All Armed", "Acquire all weapons for both Saga and Alan", False, 0),
            ("ACH_SCRATCH", "Scratch No More", "Confront Scratch at the lake shore", True, 0),
        ]
    },
    {
        "id": "manual:18e09f12914e9167",
        "name": "Dead Space (2023)",
        "variant": 7,
        "achievements": [
            ("ACH_CONCORDANCE", "Concordance Officer", "Complete the game on any difficulty", False, 1),
            ("ACH_UNTIL_THE_END", "Whole Again", "Unlock the alternate ending in New Game+", True, 0),
            ("ACH_PLASMA_ONLY", "One Gun", "Beat the game using only the Plasma Cutter", False, 0),
            ("ACH_CUT_OFF_LIMBS", "Surgeon", "Dismember 500 Necromorph limbs", False, 1),
            ("ACH_MAXED_OUT", "Maxed Out", "Fully upgrade all weapons and equipment", False, 0),
            ("ACH_ZERO_G", "Zero-G Athlete", "Survive 45 minutes in Zero-G environments", False, 1),
            ("ACH_BRUTE_FORCE", "Brute Force", "Defeat a Brute without taking damage", False, 1),
            ("ACH_MARKER_WHISPER", "Marker's Secret", "Collect all 12 Marker Fragments", True, 0),
            ("ACH_WISHBONE", "Wishbone", "Rip off a dangling limb using Kinesis", False, 1),
            ("ACH_USG_ISHIMURA", "Welcome Aboard", "Complete Chapter 1: New Arrivals", False, 1),
        ]
    },
    {
        "id": "manual:18e09f128e4e9137",
        "name": "The Great Ace Attorney Chronicles",
        "variant": 8,
        "achievements": [
            ("ACH_DEPARTURE", "The Great Departure", "Conclude the trial of John H. Watson", False, 1),
            ("ACH_HERLOCK", "The Partner and the Gentleman", "Engage in Great Deduction with Herlock Sholmes", False, 1),
            ("ACH_RUNAWAY", "The Adventure of the Runaway Room", "Expose the omnibus conspiracy in London", False, 1),
            ("ACH_MEMOIRS", "The Memoirs of the Clouded Kokoro", "Defend Soseki Natsume successfully", False, 1),
            ("ACH_UNSPEAKABLE", "The Unspeakable Story", "Unveil the mystery of the music box discs", False, 1),
            ("ACH_RESOLVE", "The Resolve of Naruhodo Ryunosuke", "Complete the dual saga of Ryunosuke and Susato", False, 1),
            ("ACH_ASUDEGA", "Armchair Detective", "Inspect every irrelevant background prop", False, 0),
            ("ACH_JURY_SUMMATION", "Summation Examination Pro", "Turn 6 guilty juror verdicts into not-guilty", False, 1),
            ("ACH_COURTROOM_SAVIOR", "No Objections!", "Win a case with full cross-examination health bar", False, 0),
            ("ACH_REAPER_CONFRONT", "Reaper of the Bailey", "Survive Lord van Zieks's decisive argument", False, 1),
        ]
    }
]

OTHER_GAMES = [
    "manual:18e09f128fce914f",  # Armored Core VI
    "manual:18e09f128f5e9148",  # A Plague Tale: Requiem
    "manual:18e09f128f8e914b",  # Age of Empires IV
    "manual:18e09f1298ee91e1",  # Xenoblade Chronicles 3
    "manual:18e09f12972e91c5",  # Super Smash Bros. Ultimate
    "manual:18e09f1295de91b0",  # Pokémon Scarlet
    "manual:18e09f12994e91e7",  # Mario Kart 8 Deluxe
    "manual:18e09f128c1e9114",  # Fate/stay night
    "manual:18e09f128b1e9104",  # Aokana
    "manual:18df209651d66a50",  # Umineko
    "manual:18e09f128b7e910a",  # CHAOS;CHILD
    "manual:18e09f128eee9141",  # VA-11 Hall-A
]

def prepare_directories():
    print(f"Creating directories in {BASE_DIR}...")
    for d in [BASE_DIR, TEST_DATA_DIR, CACHE_DIR, TEST_LIB_DIR, CAPTURES_DIR, GRAFISMOS_DIR]:
        os.makedirs(d, exist_ok=True)

def setup_config():
    print("Setting up biblioteca.json and config.json...")
    pointer = {
        "dir": TEST_LIB_DIR,
        "previous": ""
    }
    with open(os.path.join(TEST_DATA_DIR, "biblioteca.json"), "w", encoding="utf-8") as f:
        json.dump(pointer, f, indent=2)

    if os.path.isfile(SRC_CONFIG):
        with open(SRC_CONFIG, "r", encoding="utf-8") as f:
            cfg = json.load(f)
        cfg["root"] = CAPTURES_DIR
        with open(os.path.join(TEST_LIB_DIR, "config.json"), "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2)
    else:
        print("Warning: SRC_CONFIG not found, using default config")

def build_test_database():
    print("Building isolated test database...")
    if os.path.exists(DST_DB):
        try:
            os.remove(DST_DB)
        except Exception:
            pass

    src_conn = sqlite3.connect(f"file:{SRC_DB}?mode=ro", uri=True)
    src_cur = src_conn.cursor()

    dst_conn = sqlite3.connect(DST_DB)
    dst_cur = dst_conn.cursor()

    # Copy schema from source
    src_cur.execute("SELECT sql FROM sqlite_master WHERE type in ('table', 'index') AND sql IS NOT NULL")
    for row in src_cur.fetchall():
        sql = row[0]
        if "sqlite_sequence" in sql:
            continue
        try:
            dst_cur.execute(sql)
        except Exception as e:
            pass

    # Ensure migrations PRAGMA
    dst_cur.execute("PRAGMA application_id = 0x56495352;")
    dst_cur.execute("PRAGMA user_version = 13;")

    # Ensure achievements tables
    dst_cur.executescript("""
    CREATE TABLE IF NOT EXISTS achievements (
        app_id       TEXT NOT NULL,
        api_name     TEXT NOT NULL,
        display_name TEXT NOT NULL,
        description  TEXT NOT NULL DEFAULT '',
        hidden       INTEGER NOT NULL DEFAULT 0,
        icon         TEXT,
        icon_gray    TEXT,
        source       TEXT NOT NULL DEFAULT 'gse',
        PRIMARY KEY (app_id, api_name)
    );

    CREATE TABLE IF NOT EXISTS achievement_unlocks (
        app_id      TEXT NOT NULL,
        api_name    TEXT NOT NULL,
        unlocked_at INTEGER NOT NULL,
        source      TEXT NOT NULL DEFAULT 'gse',
        evidence    TEXT,
        notified    INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (app_id, api_name)
    );

    CREATE TABLE IF NOT EXISTS title_achievements_pref (
        title_id TEXT PRIMARY KEY,
        variant  INTEGER NOT NULL DEFAULT 1
    );
    """)

    all_game_ids = [g["id"] for g in SHOWCASE_GAMES] + OTHER_GAMES

    # Copy tables data for the selected games
    tables_to_filter = [
        ("games", "id"),
        ("play_stats", "game_id"),
        ("sessions", "game_id"),
        ("title_prefs", "title_id"),
        ("title_magpie", "title_id"),
        ("title_status", "title_id"),
        ("title_release", "title_id"),
        ("sgdb", "title_id"),
        ("novel_prefs", "title_id"),
    ]

    for tbl, col in tables_to_filter:
        try:
            placeholders = ",".join(["?"] * len(all_game_ids))
            src_cur.execute(f"SELECT * FROM {tbl} WHERE {col} IN ({placeholders})", all_game_ids)
            rows = src_cur.fetchall()
            if rows:
                col_count = len(rows[0])
                ins_placeholders = ",".join(["?"] * col_count)
                dst_cur.executemany(f"INSERT OR REPLACE INTO {tbl} VALUES ({ins_placeholders})", rows)
                print(f"  Copied {len(rows)} rows into {tbl}")
        except Exception as e:
            print(f"  Note: error copying {tbl}: {e}")

    # Copy sagas / saga_members
    try:
        placeholders = ",".join(["?"] * len(all_game_ids))
        src_cur.execute(f"SELECT * FROM saga_members WHERE title_id IN ({placeholders})", all_game_ids)
        members = src_cur.fetchall()
        if members:
            dst_cur.executemany("INSERT OR REPLACE INTO saga_members VALUES (?, ?, ?)", members)
            saga_ids = list(set([m[0] for m in members]))
            s_placeholders = ",".join(["?"] * len(saga_ids))
            src_cur.execute(f"SELECT * FROM sagas WHERE id IN ({s_placeholders})", saga_ids)
            sagas = src_cur.fetchall()
            if sagas:
                col_c = len(sagas[0])
                dst_cur.executemany(f"INSERT OR REPLACE INTO sagas VALUES ({','.join(['?']*col_c)})", sagas)
    except Exception as e:
        print(f"  Note sagas: {e}")

    # Copy up to 3 shots per game and copy the physical files
    print("Copying sample screenshots...")
    shots_copied = 0
    for gid in all_game_ids:
        src_cur.execute("SELECT * FROM shots WHERE title_id = ? ORDER BY date_ms DESC LIMIT 3", (gid,))
        shots = src_cur.fetchall()
        if not shots:
            continue
        col_c = len(shots[0])
        dst_cur.executemany(f"INSERT OR REPLACE INTO shots VALUES ({','.join(['?']*col_c)})", shots)
        for s in shots:
            rel_path = s[0] # e.g. PC/Alan Wake 2/xxx.png
            src_shot = os.path.join(SRC_LIB, "Capturas", rel_path)
            dst_shot = os.path.join(CAPTURES_DIR, rel_path)
            if os.path.isfile(src_shot):
                os.makedirs(os.path.dirname(dst_shot), exist_ok=True)
                try:
                    shutil.copy2(src_shot, dst_shot)
                    shots_copied += 1
                except Exception:
                    pass

    print(f"  Copied {shots_copied} screenshot files.")

    # Copy Grafismos (artwork posters, banners) for the games
    print("Copying Grafismos (box art/banners) for sample games...")
    art_copied = 0
    src_grafismos = os.path.join(SRC_LIB, "Grafismos")
    if os.path.isdir(src_grafismos):
        for gid in all_game_ids:
            # Match folder starting with manual_... or title_id
            safe_name = gid.replace(":", "_")
            for item in os.listdir(src_grafismos):
                if item.startswith(safe_name) or item.startswith(gid):
                    src_item = os.path.join(src_grafismos, item)
                    dst_item = os.path.join(GRAFISMOS_DIR, item)
                    if os.path.isdir(src_item) and not os.path.exists(dst_item):
                        shutil.copytree(src_item, dst_item)
                        art_copied += 1
    print(f"  Copied {art_copied} Grafismos folders.")

    # Populate Achievements and Unlocks for Showcase Games
    print("Inserting sample achievements and setting card variants...")
    now_ts = int(time.time())
    one_day = 86400

    for game in SHOWCASE_GAMES:
        gid = game["id"]
        variant = game["variant"]
        achs = game["achievements"]

        # Set preference
        dst_cur.execute("INSERT OR REPLACE INTO title_achievements_pref (title_id, variant) VALUES (?, ?)", (gid, variant))

        for idx, (api_name, display_name, desc, hidden, unlocked) in enumerate(achs):
            hidden_val = 1 if hidden else 0
            dst_cur.execute("""
                INSERT OR REPLACE INTO achievements (app_id, api_name, display_name, description, hidden, icon, icon_gray, source)
                VALUES (?, ?, ?, ?, ?, ?, ?, 'gse')
            """, (gid, api_name, display_name, desc, hidden_val, None, None))

            if unlocked:
                # Distribute unlock dates over the last 7 days so weekly activity charts look great
                days_ago = idx % 6
                unlock_time = now_ts - (days_ago * one_day) - (idx * 3600)
                dst_cur.execute("""
                    INSERT OR REPLACE INTO achievement_unlocks (app_id, api_name, unlocked_at, source, evidence, notified)
                    VALUES (?, ?, ?, 'gse', 'test_sample', 1)
                """, (gid, api_name, unlock_time))

    dst_conn.commit()
    dst_conn.close()
    src_conn.close()
    print("Database build complete!")

def create_launcher_scripts():
    print("Creating launcher and README...")
    bat_content = """@echo off
chcp 65001 >nul
title Visor de Capturas · Version de Prueba (Logros)
echo =======================================================================
echo          VISOR DE CAPTURAS · MODO DE PRUEBA DE LOGROS
echo =======================================================================
echo.
echo [SEGURIDAD GARANTIZADA]
echo Este ejecutable funciona en MODO AISLADO:
echo  - Lee y escribe unicamente en esta carpeta (test_data\\ y test_library\\)
echo  - Tu base de datos original en P:\\ScreenshotUL NO se toca ni se altera
echo.
echo [VARIACIONES DE INTERFAZ INCLUIDAS]
echo En la ficha de juego podras ver las 5 variantes:
echo  * Variante 1 (Clasica compacta): Elden Ring
echo  * Variante 2 (Banner cinematografico): Cyberpunk 2077
echo  * Variante 4 (Radial / Rueda de progreso): Alan Wake 2
echo  * Variante 7 (Trophy showcase): Dead Space (2023)
echo  * Variante 8 (Timeline / Actividad): The Great Ace Attorney Chronicles
echo.
echo Pulsa 'Y' en el mando (o el boton de ajustes en la tarjeta) para ciclar
echo entre las variantes en cualquier juego.
echo.
echo Iniciando visor...
echo.
start "" "%~dp0Visor-Logros-Prueba.exe" --test
"""
    with open(os.path.join(BASE_DIR, "INICIAR-PRUEBA.bat"), "w", encoding="utf-8") as f:
        f.write(bat_content)

    readme_content = """VISOR DE CAPTURAS - VERSIÓN DE PRUEBA DE LOGROS
================================================

Esta carpeta contiene una versión completa y autónoma del visor con el nuevo
sistema de logros implementado sobre una porción de prueba de tu biblioteca real.

GARANTÍA DE AISLAMIENTO:
-----------------------
- La aplicación se ejecuta con argumentos y estructura aislada (`test_data` y `test_library`).
- El visor detecta automáticamente el entorno de prueba y muestra en el título:
  "Visor de capturas · [PRUEBA DE LOGROS - BASE AISLADA]"
- Tu base de datos de producción en `P:\\ScreenshotUL\\Visor de capturas\\visor.db`
  NO es leída ni modificada en absoluto durante la prueba.
- Cualquier cambio que hagas en esta prueba (marcar logros, favoritos, variantes)
  se guarda únicamente en `test_library\\visor.db`.

JUEGOS DE MUESTRA CONFIGURADOS:
------------------------------
Cada uno de estos juegos está preconfigurado con una variante de diseño diferente:
1. ELDEN RING (PC) -> Variante 1 (Clásica compacta de 2 columnas)
2. Cyberpunk 2077 (PC) -> Variante 2 (Banner cinematográfico con barra de gradiente)
3. Alan Wake 2 (PC) -> Variante 4 (Radial / Rueda circular con progreso SVG y lista)
4. Dead Space (2023) (PC) -> Variante 7 (Trophy showcase con insignias oro/plata/bronce)
5. The Great Ace Attorney Chronicles (VN) -> Variante 8 (Timeline de actividad semanal)

Además se incluyen más de 10 juegos adicionales (Armored Core VI, Xenoblade Chronicles 3,
Super Smash Bros., Fate/stay night, Aokana, etc.) con sus estadísticas de tiempo,
capturas de muestra y grafismos.

CÓMO CAMBIAR DE VARIANTE EN CUALQUIER MOMENTO:
---------------------------------------------
- Desde la ficha del juego, puedes hacer clic en el botón de cambiar variante (icono de pincel/engranaje en la tarjeta).
- Con mando de Xbox/PlayStation: pulsa el botón 'Y' (o triángulo) mientras estás en la ficha.
- Para abrir la lista completa de logros y filtrarlos: haz clic en "Ver todos los logros" o pulsa 'A' / 'X' sobre la tarjeta.
"""
    with open(os.path.join(BASE_DIR, "LEEME-INSTRUCCIONES.txt"), "w", encoding="utf-8") as f:
        f.write(readme_content)

def main():
    prepare_directories()
    setup_config()
    build_test_database()
    create_launcher_scripts()
    print("\nAll files and database slice successfully prepared in:")
    print(BASE_DIR)

if __name__ == "__main__":
    main()

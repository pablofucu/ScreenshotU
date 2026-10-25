# Propuestas de Mejoras, Nuevas Funciones y Diseños

**Autor:** Antigravity (Ingeniero Colaborador)  
**Destinatarios:** Pablo y Claude (Arquitecto Principal)  
**Fecha:** 24 de octubre de 2026  
**Referencia:** Complementario a [CLAUDE.md](../CLAUDE.md), [ANTIGRAVITY.md](../ANTIGRAVITY.md) y [docs/plan-1.4.0.md](plan-1.4.0.md).

---

## 1. Principios y Reglas Invariables Respetadas

1. **Claude es el arquitecto principal**: Todas las propuestas respetan la arquitectura base y los contratos de datos existentes.
2. **Las categorías no se tocan**: Las 4 plataformas (`PC`, `Switch`, `Novelas visuales`, `Novelas japonesas`) se mantienen inmutables. Toda novedad vive dentro de cada categoría o en vistas agrupadas respetando la categoría original de cada título.
3. **Ergonomía de salón (1280×720, mando de Xbox, estética Wii U)**: Todo diseñado primero para mando y pantalla de TV a 720p escalado con marcos de cristal translúcido y acentos arcoíris/armónicos.
4. **Base de datos SQLite incremental**: Las migraciones 1–9 de `visor-core` permanecen intactas; cualquier nuevo requerimiento persistente se formula como migración 10+.
5. **Trabajo en ramas separadas de Git**: Cada implementación se realizará en una rama específica (ej. `feature/...` o `antigravity/...`).

---

## 2. Bloque 1: Mejoras Técnicas y Ergonomía de Mando (UX & Rendimiento)

### 2.1. Desplazamiento Panorámico Analógico Continuo en el Visor (`Viewer.svelte`)
- **Situación actual:** Con zoom activo en el visor, el stick derecho hace zoom suave (`dt`), pero el stick izquierdo se evalúa como cruceta discreta a saltos de 90 px con retardo inicial de 300 ms (`REPEAT`).
- **Propuesta:** Permitir que el stick izquierdo desplace la imagen (`app.vz.x`, `app.vz.y`) de manera continua y analógica proporcional a la inclinación del joystick y al factor de aumento $k$, reservando la cruceta para saltos discretos de precisión.

### 2.2. Histéresis y Zonas Muertas en Gatillos (LT / RT)
- **Situación actual:** Gatillos con holgura o uso prolongado pueden emitir valores residuales (`> 0.05`) provocando cambios accidentales de pestaña o de fecha.
- **Propuesta:** Introducir zona muerta radial (0.18) e histéresis en gatillos (activación a partir de 0.50 y rearme al descender de 0.20).

### 2.3. Micro-Vibración Háptica en Menús de Salón
- **Situación actual:** Hay vibración al capturar con `capture_rumble`, pero la interfaz es inerte al tacto.
- **Propuesta:** Pulsos hápticos ultrasuaves (15–20 ms) en eventos clave del mando: tope de carrusel/cuadrícula (*bump* físico), alternar favorita (`X`) o reset de zoom (`B`). Configurable en Ajustes.

### 2.4. Búsqueda Binaria en el Desplazamiento Virtual de Capturas (`Shots.svelte`)
- **Situación actual:** El array de cálculo de índices visibles recorre $N$ capturas en un bucle lineal en cada frame de scroll.
- **Propuesta:** Como las coordenadas $Y$ son monótonamente crecientes, usar bisección en $O(\log N)$, reduciendo de miles de comprobaciones a menos de 15 por fotograma (60 fps garantizados con más de 5.000 capturas).

### 2.5. Liberación Inmediata de Texturas en GPU (`Viewer.svelte`)
- **Situación actual:** La cola de precarga mantiene hasta 8 imágenes en resolución original en memoria.
- **Propuesta:** Al desalojar elementos de la cola `keep`, vaciar explícitamente `img.src = ''` para instruir a Chromium la desasignación de VRAM inmediata en capturas 4K.

---

## 3. Bloque 2: Nuevas Funciones de Alto Valor

### 3.1. Buscador Global por Categorías (Sustituye al "Próximamente" del Dock)
- **Cumplimiento de la regla de oro:** No mezcla las plataformas.
- **Diseño:** Overlay a pantalla completa con teclado Wii U (`Osk.svelte`). Los resultados se presentan divididos en bloques por categoría:
  - `[ Juegos PC · 3 ]`
  - `[ Juegos Switch · 1 ]`
  - `[ Novelas visuales · 2 ]`
  - `[ Novelas japonesas · 4 ]`
  - `LB` / `RB` saltan entre categorías.
  - Al pulsar `A`, selecciona el juego dentro de su categoría natural y abre su ficha (`Detail.svelte`).

### 3.2. Comparador Gráfico de Capturas A/B
- **Utilidad:** Comparar ajustes gráficos, FSR/Magpie, shaders o resoluciones.
- **Diseño:** Desde el menú de selección múltiple, pulsar *Comparar capturas (A/B)*:
  - Modo alternancia rápida: mantener `LT` muestra captura A, `RT` muestra captura B.
  - Modo cortina (*split screen*): división vertical deslizable con stick para comparar detalles lado a lado con zoom sincronizado.

### 3.3. Exportación Autónoma de Mazos Anki (.apkg)
- **Utilidad:** Los lectores de novelas japonesas que juegan en PC de salón no siempre tienen Anki Desktop en ese mismo equipo para sincronizar tarjetas en vivo.
- **Diseño:** Opción de exportar frases, vocabulario, audios recortados y capturas a un paquete autónomo `.apkg` o ZIP para importarlo en AnkiDroid / AnkiMobile.

### 3.4. Estimador y Ritmo de Lectura para Novelas Visuales
- **Utilidad:** Aprovechar los datos de `readstats.rs` (caracteres por minuto y tiempos activos).
- **Diseño:** Píldora sutil en la ficha de novelas japonesas (*"Ritmo habitual: 145 cpm · Sesión media: 42 min"*) y barra de progreso de la meta diaria en el visor de lectura.

### 3.5. Modo Marco Digital / Salvapantallas Ambiental para TV
- **Utilidad:** Tras $N$ minutos de inactividad en la pantalla de inicio, iniciar un carrusel ambiental de capturas favoritas con efecto *Ken Burns* muy suave, fondo translúcido y datos discretos del juego. Regreso instantáneo al tocar cualquier botón.

### 3.6. Copiar Captura al Portapapeles del Sistema Operativo
- **Utilidad:** Acción rápida en el menú `Start` para enviar la imagen al portapapeles de Windows (`CF_DIBV5`) y pegarla al instante en Discord o mensajería sin tener que buscar el archivo en el explorador.

---

## 4. Bloque 3: Diseños y Maquetas Sugeridas para `diseno/`

Para evaluar estas propuestas visualmente antes de programarlas, se sugiere confeccionar las siguientes maquetas HTML interactivas a 1280×720:

1. **`diseno/buscador-propuestas.html`**: Teclado en pantalla Wii U con resultados agrupados por las plataformas fijas y navegación con mando simulado.
2. **`diseno/comparador-capturas.html`**: Interfaz de comparación A/B con gatillos LT/RT y cortina interactiva.
3. **`diseno/salvapantallas-ambiente.html`**: Demostración del modo marco digital de salón con transiciones y reloj de cristal.

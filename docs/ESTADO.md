# Estado del proyecto

Única lista de proyectos. Se actualiza en el mismo commit que cambia el estado de algo. Revisado el 11-nov-2026 contra el historial de git (el tablero Kanvas retirado ese día estaba desfasado: daba por «propuesto» trabajo ya subido).

## En curso

| Proyecto | Plan | Hecho | Siguiente paso |
|---|---|---|---|
| Interfaz y estadísticas | [planes/interfaz-pendiente.md](planes/interfaz-pendiente.md) (anexo: [planes/resumen-semanal.md](planes/resumen-semanal.md)) | R1 (`weekReview` con tests), E1 (pestañas Semana, Terminados, Panorama), E2 y E3 (Semana con chips y columnas por categoría; pendiente probar en la app real la lectura japonesa) | E4: pestaña Terminados. Después E5–E6, R3–R6, L1–L3 (Mi lista), H1 (Inicio), C1 (Colecciones), A1–A5 (animaciones) |
| Estadísticas en una pantalla | — (rama EEU en Unificador; maqueta «Tablero», P-14) | Las 7 pestañas caben en 1280×720 sin desplazar, cada categoría en su carril o columna; Terminados grande por fases del año; revisado con mando en mock | Probarlo en la app real con la biblioteca de verdad (muchas categorías y títulos) |
| Tablet con la interfaz nueva | [planes/tablet-interfaz.md](planes/tablet-interfaz.md) | Maqueta B «En su sitio» elegida; código subido (17-nov): lienzo 1280×800, acciones táctiles en su sitio, pulsación larga, 44 px, visor con pellizco, menú sobre el juego a 16:10 | **Probar en la POCO Pad** (TCL-10) |
| Rachas con días libres | [planes/rachas.md](planes/rachas.md) | Reglas de días libres (2 cada 7, deuda pagada con lo leído de más en 7 días, meta diferida) con pruebas; pantalla de racha nueva (velas); Estadísticas › Rachas con la pantalla antigua; Inicio, menú sobre el juego, avisos y tablet; hucha retirada | Probarlo en la app real (pantalla de racha, aviso «¡Día libre recuperado!», tablet) y, si va bien, archivar el plan |

## Siguiente (empezados, con trabajo pendiente)

| Proyecto | Plan | Hecho | Qué falta |
|---|---|---|---|
| Magpie y táctil en la tablet | [planes/magpie-tablet.md](planes/magpie-tablet.md) | A1, A2, B0, B1, B2 | A5 (pruebas de streaming y nitidez, las hace el usuario), A3 (modo de escalado **por juego**, decidido el 12-nov), A4 (recalcular al reconectar), B3 (explicarlo en Ajustes) |
| Tablet: menú sobre el juego y volver a la partida | [archivo/plan-tablet.md](archivo/plan-tablet.md), [archivo/plan-volver-partida.md](archivo/plan-volver-partida.md) | Código subido (T1–T4, volver a la partida, menú con la interfaz del overlay) | **Probar en la tablet real** y compilar la APK. Por confirmar si siguen pendientes: ocultar en la tablet las opciones que solo valen en el PC y la adaptación táctil (objetivos de 44 px, pulsación larga) |
| Multi-PC | [planes/multi-pc.md](planes/multi-pc.md) | Fases 1–5 (dispositivos, cliente PC, instalaciones en clientes, partidas de viaje, Tailscale) | Prueba con dos PCs en la red (1G). La fase 6 (nube personal) queda aparcada (12-nov) |
| Acento vivo | [planes/acento-vivo.md](planes/acento-vivo.md) | Pasos 1–4 (repaso visual sin fallos, 15-nov) | Cierre: verificar tokens en todos los componentes (AVY-2) y decidir el texto blanco sobre `--arc-*` claros |
| Guía de diseño | [guia-diseno.md](guia-diseno.md) | Fases 0–6 de la migración | Fase 7 |

## Decisiones que esperan al usuario

- **Inicio (H1):** confirmar los retoques A–D de [maquetas/menu-principal/](maquetas/menu-principal/).

## Aparcado

- **Colecciones (C1):** aparcado el 12-nov. Quedan las maquetas A–E en [maquetas/colecciones/](maquetas/colecciones/) sin elegir.
- **Nube personal (Multi-PC, fase 6):** aparcada el 12-nov.

- **Switch, importar capturas al conectar la consola:** solo estudio (MTP, microSD, hash → id de título). El tiempo de juego de Switch está terminado y publicado, y desde el 22-nov cuenta en todas las estadísticas por fechas (días sin hora: suman tiempo y días, no partidas ni horas). Capturas por la cuenta de Nintendo: estudio del 22-nov, sin API pública; solo Switch 2 sube sola (30 días, 100 ficheros) al servidor `nega` (media-dl.wc.lp1.nega.srv.nintendo.net), sin rutas ni permisos documentados. Haría falta mirar el tráfico de la app Nintendo Switch.
- **Ideas con maqueta y sin plan:** comparador de capturas A/B y salvapantallas de ambiente ([maquetas/ideas/](maquetas/ideas/)); exportar .apkg y ritmo de lectura ([archivo/propuestas-antigravity.md](archivo/propuestas-antigravity.md)).
- **Propuestas nuevas sin decidir:** [propuestas-mejoras.md](propuestas-mejoras.md).

## Terminado (planes en `archivo/`)

Arreglos de interfaz (14-nov, ADI-1 a 16; [archivo/arreglos-interfaz.md](archivo/arreglos-interfaz.md)), Emparejamiento nuevo (12-nov: solicitud con 4 cifras o billete de un solo uso, Conexiones en el PC principal y en el cliente, tablet con Artemis; [archivo/emparejamiento.md](archivo/emparejamiento.md); falta probarlo con dispositivos reales), Revisión del 9 de noviembre (fases 2 y 3), metas de juegos, menú del juego, overlay (v3 es el último), tiempo de juego de Switch (fases 1–4), streaming a la tablet T1–T4, lectura de estudio, versiones 1.4.0 y 1.7.0.

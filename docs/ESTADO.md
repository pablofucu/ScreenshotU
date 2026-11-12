# Estado del proyecto

Única lista de proyectos. Se actualiza en el mismo commit que cambia el estado de algo. Revisado el 11-nov-2026 contra el historial de git (el tablero Kanvas retirado ese día estaba desfasado: daba por «propuesto» trabajo ya subido).

## En curso

| Proyecto | Plan | Hecho | Siguiente paso |
|---|---|---|---|
| Interfaz y estadísticas | [planes/interfaz-pendiente.md](planes/interfaz-pendiente.md) (anexo: [planes/resumen-semanal.md](planes/resumen-semanal.md)) | R1 (`weekReview` con tests) y E1 (pestañas Semana, Terminados, Panorama) | E2: contenido de la pestaña Semana. Después E3–E6, R3–R6, L1–L3 (Mi lista), H1 (Inicio), C1 (Colecciones), A1–A5 (animaciones) |

## Siguiente (empezados, con trabajo pendiente)

| Proyecto | Plan | Hecho | Qué falta |
|---|---|---|---|
| Magpie y táctil en la tablet | [planes/magpie-tablet.md](planes/magpie-tablet.md) | A1, A2, B0, B1, B2 | A5 (pruebas de streaming y nitidez, las hace el usuario), A3 (modo por juego y destino), A4 (recalcular al reconectar), B3 (explicarlo en Ajustes) |
| Tablet: menú sobre el juego y volver a la partida | [archivo/plan-tablet.md](archivo/plan-tablet.md), [archivo/plan-volver-partida.md](archivo/plan-volver-partida.md) | Código subido (T1–T4, volver a la partida, menú con la interfaz del overlay) | **Probar en la tablet real** y compilar la APK. Por confirmar si siguen pendientes: ocultar en la tablet las opciones que solo valen en el PC y la adaptación táctil (objetivos de 44 px, pulsación larga) |
| Emparejamiento nuevo | [planes/emparejamiento.md](planes/emparejamiento.md) | Plan y decisiones (12-nov) | Fase 0 (estudio de Apollo, Moonlight, Artemis y la notificación) y fase M (maquetas) |
| Multi-PC | [planes/multi-pc.md](planes/multi-pc.md) | Fases 1–5 (dispositivos, cliente PC, instalaciones en clientes, partidas de viaje, Tailscale) | Prueba con dos PCs en la red (1G) y fase 6 (nube personal) |
| Acento vivo | [planes/acento-vivo.md](planes/acento-vivo.md) | Pasos 1–3 | Paso 4: repaso visual con los 7 tonos |
| Guía de diseño | [guia-diseno.md](guia-diseno.md) | Fases 0–6 de la migración | Fase 7 |

## Decisiones que esperan al usuario

- **Colecciones (C1):** elegir una de las maquetas A–E de [maquetas/colecciones/](maquetas/colecciones/).
- **Inicio (H1):** confirmar los retoques A–D de [maquetas/menu-principal/](maquetas/menu-principal/).

## Aparcado

- **Switch, importar capturas al conectar la consola:** solo estudio (MTP, microSD, hash → id de título). El tiempo de juego de Switch está terminado y publicado.
- **Ideas con maqueta y sin plan:** comparador de capturas A/B y salvapantallas de ambiente ([maquetas/ideas/](maquetas/ideas/)); exportar .apkg y ritmo de lectura ([archivo/propuestas-antigravity.md](archivo/propuestas-antigravity.md)).
- **Propuestas nuevas sin decidir:** [propuestas-mejoras.md](propuestas-mejoras.md).

## Terminado (planes en `archivo/`)

Revisión del 9 de noviembre (fases 2 y 3), metas de juegos, menú del juego, overlay (v3 es el último), tiempo de juego de Switch (fases 1–4), streaming a la tablet T1–T4, lectura de estudio, versiones 1.4.0 y 1.7.0.

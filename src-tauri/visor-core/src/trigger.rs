//! Maquina de estados del atajo del menu sobre el juego. Teclado, mando y raton comparten el mismo gesto:
//! pulsar abre o cierra; mantener fuerza el cierre del juego (como el boton de encendido de una consola).
//! Es pura (el tiempo llega de fuera) para poder probarla sin ventanas.

/// Hasta aqui la pulsacion es corta; a partir de aqui se enseña el progreso de mantener.
pub const TAP_MS: u64 = 400;
/// Tiempo de mantener por defecto para forzar el cierre.
pub const HOLD_MS: u64 = 1500;
/// Tiempos de mantener que se pueden elegir en Ajustes.
pub const HOLD_OPTIONS: &[u64] = &[1000, 1500, 2000, 3000];

/// Ritmo de los toques de vibracion mientras se mantiene sin poder enseñar el menu.
const TICK_SLOW_MS: u64 = 260;
const TICK_FAST_MS: u64 = 90;

/// Situacion en el momento de pulsar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerCtx {
    /// El menu puede salir (hay partida y no es pantalla completa exclusiva).
    pub can_show: bool,
    pub is_open: bool,
    /// Mantener fuerza el cierre.
    pub hold_force: bool,
    pub hold_ms: u64,
}

/// Lo que hay que hacer como respuesta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerAct {
    None,
    Open,
    Close,
    /// «Aqui no se puede abrir»: dos toques de vibracion.
    Warn,
    HoldStart,
    /// Progreso de mantener (0..1).
    HoldProgress(f32),
    /// Toque de vibracion (mantener sin menu visible).
    HoldTick,
    Force,
    HoldCancel,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    t0: u64,
    ctx: TriggerCtx,
    /// Ya se ha enseñado el progreso de mantener.
    holding: bool,
    next_tick: u64,
    /// Ya se forzo el cierre: lo que quede de esta pulsacion no hace nada.
    forced: bool,
}

/// Una pulsacion cada vez: un segundo `down` mientras hay otra activa (teclado y mando a la vez) se ignora.
#[derive(Debug, Default)]
pub struct TriggerFsm {
    press: Option<Press>,
}

impl TriggerFsm {
    pub const fn new() -> Self {
        TriggerFsm { press: None }
    }

    pub fn is_active(&self) -> bool {
        self.press.is_some()
    }

    pub fn down(&mut self, now: u64, ctx: TriggerCtx) -> TriggerAct {
        if self.press.is_some() {
            return TriggerAct::None;
        }
        self.press = Some(Press { t0: now, ctx, holding: false, next_tick: now + TAP_MS, forced: false });
        if ctx.can_show && !ctx.is_open {
            TriggerAct::Open
        } else {
            TriggerAct::None
        }
    }

    /// Se llama cada ~30 ms mientras esta pulsado. Devuelve un solo acto por llamada.
    pub fn tick(&mut self, now: u64) -> TriggerAct {
        let Some(p) = self.press.as_mut() else { return TriggerAct::None };
        if p.forced || !p.ctx.hold_force {
            return TriggerAct::None;
        }
        let dt = now.saturating_sub(p.t0);
        if dt >= p.ctx.hold_ms {
            p.forced = true;
            return TriggerAct::Force;
        }
        if dt < TAP_MS {
            return TriggerAct::None;
        }
        if !p.ctx.can_show {
            if now < p.next_tick {
                return TriggerAct::None;
            }
            // De cada 260 ms a cada 90 ms segun se acerca el final.
            let span = p.ctx.hold_ms.saturating_sub(TAP_MS).max(1);
            let f = (dt - TAP_MS).min(span) as f32 / span as f32;
            let gap = TICK_SLOW_MS as f32 - (TICK_SLOW_MS - TICK_FAST_MS) as f32 * f;
            p.next_tick = now + gap as u64;
            return TriggerAct::HoldTick;
        }
        if !p.holding {
            p.holding = true;
            return TriggerAct::HoldStart;
        }
        TriggerAct::HoldProgress(dt as f32 / p.ctx.hold_ms as f32)
    }

    pub fn up(&mut self, now: u64) -> TriggerAct {
        let Some(p) = self.press.take() else { return TriggerAct::None };
        if p.forced {
            return TriggerAct::None;
        }
        let dt = now.saturating_sub(p.t0);
        // Sin «mantener» cualquier pulsacion es corta, dure lo que dure.
        if dt < TAP_MS || !p.ctx.hold_force {
            return if !p.ctx.can_show {
                TriggerAct::Warn
            } else if p.ctx.is_open {
                TriggerAct::Close
            } else {
                TriggerAct::None
            };
        }
        TriggerAct::HoldCancel
    }
}

/// Tiempo de mantener valido a partir del de la configuracion.
pub fn normalize_hold_ms(ms: u64) -> u64 {
    if HOLD_OPTIONS.contains(&ms) {
        ms
    } else {
        HOLD_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn ctx(can_show: bool, is_open: bool) -> TriggerCtx {
        TriggerCtx { can_show, is_open, hold_force: true, hold_ms: HOLD_MS }
    }

    /// Avanza de 30 en 30 ms hasta `to` y devuelve los actos distintos de `None`.
    fn run(f: &mut TriggerFsm, from: u64, to: u64) -> Vec<TriggerAct> {
        let mut v = Vec::new();
        let mut t = from;
        while t <= to {
            match f.tick(t) {
                TriggerAct::None => {}
                a => v.push(a),
            }
            t += 30;
        }
        v
    }

    #[test]
    fn closed_tap_opens_and_stays_open() {
        let mut f = TriggerFsm::new();
        assert_eq!(f.down(0, ctx(true, false)), TriggerAct::Open);
        assert!(run(&mut f, 30, 300).is_empty());
        assert_eq!(f.up(300), TriggerAct::None);
        assert!(!f.is_active());
    }

    #[test]
    fn open_tap_closes() {
        let mut f = TriggerFsm::new();
        assert_eq!(f.down(0, ctx(true, true)), TriggerAct::None);
        assert_eq!(f.up(200), TriggerAct::Close);
    }

    #[test]
    fn hold_released_midway_cancels_and_keeps_menu() {
        for open in [false, true] {
            let mut f = TriggerFsm::new();
            f.down(0, ctx(true, open));
            let acts = run(&mut f, 30, 1000);
            assert_eq!(acts[0], TriggerAct::HoldStart);
            assert!(matches!(acts[1], TriggerAct::HoldProgress(p) if p > 0.2 && p < 0.4));
            assert_eq!(f.up(1000), TriggerAct::HoldCancel);
        }
    }

    #[test]
    fn hold_reaches_force_once() {
        for c in [ctx(true, false), ctx(true, true), ctx(false, false)] {
            let mut f = TriggerFsm::new();
            f.down(0, c);
            let acts = run(&mut f, 30, 3000);
            assert_eq!(acts.iter().filter(|a| **a == TriggerAct::Force).count(), 1);
            assert_eq!(f.up(3000), TriggerAct::None);
        }
    }

    #[test]
    fn exclusive_tap_warns_and_hold_ticks_faster() {
        let mut f = TriggerFsm::new();
        assert_eq!(f.down(0, ctx(false, false)), TriggerAct::None);
        assert_eq!(f.up(200), TriggerAct::Warn);

        let mut f = TriggerFsm::new();
        f.down(0, ctx(false, false));
        let mut ticks = Vec::new();
        let mut t = 30;
        while t < HOLD_MS {
            if f.tick(t) == TriggerAct::HoldTick {
                ticks.push(t);
            }
            t += 10;
        }
        assert!(ticks.len() >= 5, "{ticks:?}");
        assert!(ticks[0] >= TAP_MS);
        let first = ticks[1] - ticks[0];
        let last = ticks[ticks.len() - 1] - ticks[ticks.len() - 2];
        assert!(last < first, "{ticks:?}");
        assert_eq!(f.up(HOLD_MS - 100), TriggerAct::HoldCancel);
    }

    #[test]
    fn hold_force_off_is_always_a_tap() {
        let c = TriggerCtx { hold_force: false, ..ctx(true, true) };
        let mut f = TriggerFsm::new();
        f.down(0, c);
        assert!(run(&mut f, 30, 5000).is_empty());
        assert_eq!(f.up(5000), TriggerAct::Close);

        let c = TriggerCtx { hold_force: false, ..ctx(false, false) };
        let mut f = TriggerFsm::new();
        f.down(0, c);
        assert!(run(&mut f, 30, 5000).is_empty());
        assert_eq!(f.up(5000), TriggerAct::Warn);
    }

    #[test]
    fn second_trigger_is_ignored_while_active() {
        let mut f = TriggerFsm::new();
        assert_eq!(f.down(0, ctx(true, false)), TriggerAct::Open);
        assert_eq!(f.down(50, ctx(true, true)), TriggerAct::None);
        assert_eq!(f.up(100), TriggerAct::None);
        // La subida del segundo disparador ya no encuentra pulsacion.
        assert_eq!(f.up(120), TriggerAct::None);
    }

    #[test]
    fn hold_time_is_configurable() {
        let c = TriggerCtx { hold_ms: 3000, ..ctx(true, true) };
        let mut f = TriggerFsm::new();
        f.down(0, c);
        assert!(!run(&mut f, 30, 2900).contains(&TriggerAct::Force));
        assert_eq!(f.tick(3000), TriggerAct::Force);
    }

    #[test]
    fn hold_ms_normalization() {
        assert_eq!(normalize_hold_ms(2000), 2000);
        assert_eq!(normalize_hold_ms(0), HOLD_MS);
        assert_eq!(normalize_hold_ms(99999), HOLD_MS);
    }
}

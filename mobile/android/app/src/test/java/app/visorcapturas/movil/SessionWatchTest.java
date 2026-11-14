package app.visorcapturas.movil;

import org.junit.Test;
import static org.junit.Assert.*;

public class SessionWatchTest {
    @Test
    public void vuelveAlVisorAlTerminarJuegoOAlCancelarPendiente() {
        SessionWatch w = new SessionWatch();
        assertEquals(SessionWatch.Action.CONTINUE, w.step(true, "pending", 100, true));
        assertEquals(SessionWatch.Action.ACTIVE, w.step(true, "playing", 100, true));
        assertEquals(SessionWatch.Action.RETURN_TO_APP, w.step(true, "idle", 100, true));

        SessionWatch cancelado = new SessionWatch();
        assertEquals(SessionWatch.Action.CONTINUE, cancelado.step(true, "pending", 100, true));
        assertEquals(SessionWatch.Action.RETURN_TO_APP, cancelado.step(true, "idle", 100, true));
    }

    @Test
    public void vuelveAlVisorSiCaeLaConexionDelHostYResisteCortesBreves() {
        SessionWatch w = new SessionWatch();
        assertEquals(SessionWatch.Action.ACTIVE, w.step(true, "playing", -1, true));
        for (int i = 0; i < 4; i++) {
            assertEquals(SessionWatch.Action.CONTINUE, w.step(false, "", -1, true));
        }
        // Un fallo suelto se recupera cuando vuelve la red
        assertEquals(SessionWatch.Action.ACTIVE, w.step(true, "playing", -1, true));
        for (int i = 0; i < 4; i++) {
            assertEquals(SessionWatch.Action.CONTINUE, w.step(false, "", -1, true));
        }
        assertEquals(SessionWatch.Action.RETURN_TO_APP, w.step(false, "", -1, true));
    }

    @Test
    public void sinPartidaActivaFueraDeArtemisSeDetiene() {
        SessionWatch w = new SessionWatch();
        assertEquals(SessionWatch.Action.CONTINUE, w.step(true, "idle", 50, true));
        assertEquals(SessionWatch.Action.STOP, w.step(true, "idle", 50, false));
    }
}

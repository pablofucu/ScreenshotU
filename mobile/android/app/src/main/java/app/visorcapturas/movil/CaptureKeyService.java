package app.visorcapturas.movil;

import android.accessibilityservice.AccessibilityService;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityEvent;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/**
 * Mientras Moonlight esta en primer plano, la tecla de bajar volumen hace una captura en el PC
 * (POST /api/v1/capture). Fuera de Moonlight no toca ninguna tecla.
 */
public class CaptureKeyService extends AccessibilityService {
    private volatile String foreground = "";
    /** El ACTION_UP de una pulsacion que hemos consumido tambien se traga. */
    private boolean swallowUp = false;

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (event.getEventType() != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return;
        CharSequence p = event.getPackageName();
        // El panel de volumen y las notificaciones son del sistema: no cambian la app en primer plano.
        if (p == null || p.toString().equals("com.android.systemui")) return;
        foreground = p.toString();
    }

    @Override
    public void onInterrupt() {
    }

    @Override
    protected boolean onKeyEvent(KeyEvent e) {
        if (e.getKeyCode() != KeyEvent.KEYCODE_VOLUME_DOWN) return false;
        if (e.getAction() == KeyEvent.ACTION_UP) {
            if (swallowUp) {
                swallowUp = false;
                return true;
            }
            return false;
        }
        if (!StreamingPlugin.isMoonlight(foreground)) return false;
        swallowUp = true;
        if (e.getRepeatCount() == 0) capture();
        return true;
    }

    private void capture() {
        SharedPreferences sp = getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE);
        final String base = sp.getString("base", "");
        final String code = sp.getString("code", "");
        new Thread(() -> {
            boolean ok = false;
            if (!base.isEmpty() && !code.isEmpty()) {
                HttpURLConnection c = null;
                try {
                    c = (HttpURLConnection) new URL(base + "/api/v1/capture").openConnection();
                    c.setRequestMethod("POST");
                    c.setConnectTimeout(3000);
                    c.setReadTimeout(5000);
                    c.setDoOutput(true);
                    c.setFixedLengthStreamingMode(0);
                    c.setRequestProperty("Authorization", "Bearer " + code);
                    OutputStream o = c.getOutputStream();
                    o.close();
                    ok = c.getResponseCode() == 200;
                } catch (Exception ignored) {
                } finally {
                    if (c != null) c.disconnect();
                }
            }
            vibrate(ok);
        }, "visor-capture").start();
    }

    /** Una vibracion corta si salio bien; dos si fallo. */
    private void vibrate(boolean ok) {
        Vibrator v = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
        if (v == null || !v.hasVibrator()) return;
        long[] pattern = ok ? new long[] { 0, 60 } : new long[] { 0, 80, 90, 80 };
        v.vibrate(VibrationEffect.createWaveform(pattern, -1));
    }
}

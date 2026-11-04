package app.visorcapturas.movil;

import android.accessibilityservice.AccessibilityService;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.graphics.drawable.GradientDrawable;
import android.os.Handler;
import android.os.Looper;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowManager;
import android.view.accessibility.AccessibilityEvent;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/**
 * Mientras Moonlight/Artemis esta en primer plano: volumen abajo hace una captura en el PC
 * (POST /api/v1/capture) y volumen arriba abre un menu pequeno arriba (capturar, cerrar el juego, volver a la app).
 * Fuera de Moonlight no toca ninguna tecla.
 */
public class CaptureKeyService extends AccessibilityService {
    private volatile String foreground = "";
    /** El ACTION_UP de una pulsacion que hemos consumido tambien se traga (uno por tecla). */
    private boolean swallowDownUp = false;
    private boolean swallowVolUpUp = false;

    private final Handler ui = new Handler(Looper.getMainLooper());
    private View menu;
    private TextView closeBtn;
    private final Runnable hideMenu = this::removeMenu;

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (event.getEventType() != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return;
        CharSequence p = event.getPackageName();
        // El panel de volumen y las notificaciones son del sistema: no cambian la app en primer plano.
        if (p == null || p.toString().equals("com.android.systemui")) return;
        foreground = p.toString();
        // Si el menu estaba abierto y otra app pasa a primer plano, se quita.
        if (menu != null && !StreamingPlugin.isMoonlight(foreground)) ui.post(this::removeMenu);
    }

    @Override
    public void onInterrupt() {
    }

    @Override
    protected boolean onKeyEvent(KeyEvent e) {
        int code = e.getKeyCode();
        if (code != KeyEvent.KEYCODE_VOLUME_DOWN && code != KeyEvent.KEYCODE_VOLUME_UP) return false;
        boolean down = code == KeyEvent.KEYCODE_VOLUME_DOWN;
        if (e.getAction() == KeyEvent.ACTION_UP) {
            boolean s = down ? swallowDownUp : swallowVolUpUp;
            if (s) {
                if (down) swallowDownUp = false; else swallowVolUpUp = false;
                return true;
            }
            return false;
        }
        if (!StreamingPlugin.isMoonlight(foreground)) return false;
        if (down) swallowDownUp = true; else swallowVolUpUp = true;
        if (e.getRepeatCount() == 0) {
            if (down) capture(); else ui.post(this::toggleMenu);
        }
        return true;
    }

    /* ---------------------------------------------------------------- menu */

    private void toggleMenu() {
        if (menu != null) removeMenu(); else showMenu();
    }

    private void removeMenu() {
        ui.removeCallbacks(hideMenu);
        if (menu == null) return;
        try {
            ((WindowManager) getSystemService(Context.WINDOW_SERVICE)).removeView(menu);
        } catch (Exception ignored) {
        }
        menu = null;
        closeBtn = null;
    }

    private TextView button(String text, View.OnClickListener l) {
        TextView t = new TextView(this);
        t.setText(text);
        t.setTextColor(Color.WHITE);
        t.setTextSize(15);
        t.setGravity(Gravity.CENTER);
        int pad = (int) (12 * getResources().getDisplayMetrics().density);
        t.setPadding(pad * 2, pad, pad * 2, pad);
        t.setOnClickListener(l);
        return t;
    }

    private void showMenu() {
        removeMenu();
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(Color.argb(235, 24, 24, 32));
        bg.setCornerRadius(24 * getResources().getDisplayMetrics().density);
        row.setBackground(bg);
        row.addView(button("Capturar", v -> {
            capture();
            ui.removeCallbacks(hideMenu);
            ui.postDelayed(hideMenu, 1500);
        }));
        closeBtn = button("Cerrar juego", v -> closeGame());
        row.addView(closeBtn);
        row.addView(button("Volver a la app", v -> {
            removeMenu();
            returnToApp();
        }));
        WindowManager.LayoutParams lp = new WindowManager.LayoutParams(
                WindowManager.LayoutParams.WRAP_CONTENT, WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE | WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
                PixelFormat.TRANSLUCENT);
        lp.gravity = Gravity.TOP | Gravity.CENTER_HORIZONTAL;
        lp.y = (int) (16 * getResources().getDisplayMetrics().density);
        try {
            ((WindowManager) getSystemService(Context.WINDOW_SERVICE)).addView(row, lp);
            menu = row;
            ui.postDelayed(hideMenu, 4000);
        } catch (Exception ignored) {
        }
    }

    private void closeGame() {
        ui.removeCallbacks(hideMenu);
        final boolean force = closeBtn != null && "Forzar cierre".contentEquals(closeBtn.getText());
        new Thread(() -> {
            String r = post("/api/v1/close", force ? "{\"force\":true}" : "{}");
            boolean ok = r != null;
            vibrate(ok);
            ui.post(() -> {
                if (menu == null) return;
                if (ok && r.contains("\"asked\"")) {
                    // El juego mostro un dialogo o tarda: la siguiente pulsacion fuerza el cierre.
                    if (closeBtn != null) closeBtn.setText("Forzar cierre");
                    ui.postDelayed(hideMenu, 5000);
                } else {
                    removeMenu();
                }
            });
        }, "visor-close").start();
    }

    private void returnToApp() {
        Intent i = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (i == null) return;
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        try {
            startActivity(i);
        } catch (Exception ignored) {
        }
    }

    /* ---------------------------------------------------------------- red */

    private void capture() {
        new Thread(() -> vibrate(post("/api/v1/capture", null) != null), "visor-capture").start();
    }

    /** POST al visor con el codigo guardado. Devuelve el cuerpo si fue 200, o null si fallo. Bloquea: hilo aparte. */
    private String post(String path, String json) {
        SharedPreferences sp = getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE);
        String base = sp.getString("base", "");
        String code = sp.getString("code", "");
        if (base.isEmpty() || code.isEmpty()) return null;
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(base + path).openConnection();
            c.setRequestMethod("POST");
            c.setConnectTimeout(3000);
            c.setReadTimeout(5000);
            c.setDoOutput(true);
            c.setRequestProperty("Authorization", "Bearer " + code);
            byte[] body = json == null ? new byte[0] : json.getBytes("UTF-8");
            if (json != null) c.setRequestProperty("Content-Type", "application/json");
            c.setFixedLengthStreamingMode(body.length);
            OutputStream o = c.getOutputStream();
            o.write(body);
            o.close();
            if (c.getResponseCode() != 200) return null;
            BufferedReader r = new BufferedReader(new InputStreamReader(c.getInputStream(), "UTF-8"));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            r.close();
            return sb.toString();
        } catch (Exception ignored) {
            return null;
        } finally {
            if (c != null) c.disconnect();
        }
    }

    /** Una vibracion corta si salio bien; dos si fallo. */
    private void vibrate(boolean ok) {
        Vibrator v = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
        if (v == null || !v.hasVibrator()) return;
        long[] pattern = ok ? new long[] { 0, 60 } : new long[] { 0, 80, 90, 80 };
        v.vibrate(VibrationEffect.createWaveform(pattern, -1));
    }
}

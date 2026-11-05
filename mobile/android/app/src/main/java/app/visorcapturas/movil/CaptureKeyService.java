package app.visorcapturas.movil;

import android.accessibilityservice.AccessibilityService;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.graphics.drawable.GradientDrawable;
import android.os.Build;
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

import org.json.JSONObject;

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
        // Nuestro propio menu flotante tambien genera un evento con este paquete: no es una app nueva en primer plano.
        if (p.toString().equals(getPackageName())) {
            CharSequence cn = event.getClassName();
            if (menu != null || cn == null || !cn.toString().contains("Activity")) return;
        }
        boolean was = StreamingPlugin.isMoonlight(foreground);
        foreground = p.toString();
        if (!was && StreamingPlugin.isMoonlight(foreground)) startWatch();
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

    /* ---------------------------------------------------- fin de sesion */

    private volatile boolean watching = false;

    /** Consulta la sesion del PC: {estado, endedMs de la ultima} o null si no hay respuesta. */
    private Object[] sessionState() {
        String body = get("/api/v1/session");
        if (body == null) return null;
        try {
            JSONObject j = new JSONObject(body);
            JSONObject last = j.optJSONObject("last");
            return new Object[] { j.optString("state", "idle"), last == null ? -1L : last.optLong("endedMs", -1) };
        } catch (Exception e) {
            return null;
        }
    }

    /**
     * Mientras Artemis siga delante, pregunta al PC por la sesion. Cuando ve una partida y despues el reposo con un
     * resumen nuevo, la partida acabo: se trae esta app al frente (o una notificacion si Android lo bloquea).
     */
    private void startWatch() {
        if (watching) return;
        watching = true;
        new Thread(() -> {
            try {
                long baseline = -2;
                boolean seen = false;
                boolean done = false;
                boolean front = true;
                while (front) {
                    front = StreamingPlugin.isMoonlight(foreground);
                    Object[] s = sessionState();
                    if (s != null) {
                        String state = (String) s[0];
                        long ended = (Long) s[1];
                        if (baseline == -2) baseline = ended;
                        if ("starting".equals(state) || "playing".equals(state)) seen = true;
                        else if (seen && "idle".equals(state) && ended != baseline) {
                            done = true;
                            break;
                        }
                    }
                    if (!front) break;
                    try { Thread.sleep(2000); } catch (InterruptedException ignored) { }
                }
                // Artemis puede cerrarse solo al acabar la sesion: una consulta mas antes de irse.
                if (!done && seen) {
                    try { Thread.sleep(1500); } catch (InterruptedException ignored) { }
                    Object[] s = sessionState();
                    done = s != null && "idle".equals(s[0]) && (Long) s[1] != baseline;
                }
                if (done) ui.post(this::sessionEnded);
            } finally {
                watching = false;
            }
        }, "visor-watch").start();
    }

    private void sessionEnded() {
        removeMenu();
        returnToApp();
        // Android bloquea en silencio el arranque desde segundo plano en algunos dispositivos: si no funciono, aviso.
        ui.postDelayed(() -> {
            if (!getPackageName().equals(foreground)) notifyEnded();
        }, 1500);
    }

    private void notifyEnded() {
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        if (nm == null) return;
        if (Build.VERSION.SDK_INT >= 26) {
            nm.createNotificationChannel(new NotificationChannel("session", "Fin de sesión", NotificationManager.IMPORTANCE_HIGH));
        }
        Intent i = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (i == null) return;
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        PendingIntent pi = PendingIntent.getActivity(this, 0, i, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Builder b = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(this, "session") : new Notification.Builder(this);
        b.setSmallIcon(getApplicationInfo().icon).setContentTitle("Sesión terminada").setContentText("Toca para ver el resumen").setContentIntent(pi).setAutoCancel(true);
        nm.notify(4101, b.build());
    }

    /* ---------------------------------------------------------------- red */

    private void capture() {
        new Thread(() -> vibrate(post("/api/v1/capture", null) != null), "visor-capture").start();
    }

    /** GET al visor con el codigo guardado. Devuelve el cuerpo si fue 200, o null. Bloquea: hilo aparte. */
    private String get(String path) {
        SharedPreferences sp = getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE);
        String base = sp.getString("base", "");
        String code = sp.getString("code", "");
        if (base.isEmpty() || code.isEmpty()) return null;
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(base + path).openConnection();
            c.setConnectTimeout(3000);
            c.setReadTimeout(5000);
            c.setRequestProperty("Authorization", "Bearer " + code);
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

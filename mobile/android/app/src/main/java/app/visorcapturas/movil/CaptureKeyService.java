package app.visorcapturas.movil;

import android.accessibilityservice.AccessibilityService;
import android.annotation.SuppressLint;
import android.net.Uri;
import android.webkit.JavascriptInterface;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.PixelFormat;
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

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/**
 * Mientras Moonlight/Artemis esta en primer plano: volumen abajo hace una captura en el PC
 * (POST /api/v1/capture) y volumen arriba abre el menu del juego (la pagina del overlay del PC en un WebView).
 * Fuera de Moonlight no toca ninguna tecla.
 */
public class CaptureKeyService extends AccessibilityService {
    private volatile String foreground = "";
    /** El ACTION_UP de una pulsacion que hemos consumido tambien se traga (uno por tecla). */
    private boolean swallowDownUp = false;
    private boolean swallowVolUpUp = false;

    private final Handler ui = new Handler(Looper.getMainLooper());
    private View menu;
    private WebView web;

    /** Se acaba de ver la pantalla del stream de Artemis: si ahora sale su lista, el usuario ha salido con atras. */
    private boolean leftStream;

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
        // Atras desde el stream: si lo que aparece justo despues es la lista de Artemis, se vuelve al visor.
        // Solo se arma tras ver la pantalla del stream; abrir Artemis a mano no saca de el.
        if (StreamingPlugin.isMoonlight(p.toString())) {
            String cls = String.valueOf(event.getClassName());
            if (cls.endsWith(".Game")) leftStream = true;
            else if (leftStream && (cls.endsWith(".PcView") || cls.endsWith(".AppView"))) {
                leftStream = false;
                ui.post(this::returnToApp);
            }
        } else {
            leftStream = false;
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
            if (down) capture(); else toggleOverlay();
        }
        return true;
    }

    /* ---------------------------------------------------------------- menu */

    private void toggleMenu() {
        if (menu != null) removeMenu(); else showMenu();
    }

    private void removeMenu() {
        if (menu == null) return;
        try {
            ((WindowManager) getSystemService(Context.WINDOW_SERVICE)).removeView(menu);
        } catch (Exception ignored) {
        }
        menu = null;
    }

    /** Puente JS del menu: la pagina pide la configuracion del servidor y avisa de cerrar o volver. */
    private final class QmBridge {
        @JavascriptInterface
        public String config() {
            SharedPreferences sp = getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE);
            try {
                URL u = new URL(sp.getString("base", ""));
                String code = sp.getString("code", "");
                JSONObject o = new JSONObject();
                o.put("host", u.getHost());
                o.put("port", u.getPort() > 0 ? u.getPort() : 80);
                o.put("code", code);
                o.put("token", code);
                return o.toString();
            } catch (Exception e) {
                return "{}";
            }
        }

        @JavascriptInterface
        public void close() {
            ui.post(CaptureKeyService.this::removeMenu);
        }

        @JavascriptInterface
        public void back() {
            ui.post(() -> {
                removeMenu();
                returnToApp();
            });
        }
    }

    private static String mime(String path) {
        String p = path.toLowerCase();
        if (p.endsWith(".html")) return "text/html";
        if (p.endsWith(".js") || p.endsWith(".mjs")) return "text/javascript";
        if (p.endsWith(".css")) return "text/css";
        if (p.endsWith(".json")) return "application/json";
        if (p.endsWith(".svg")) return "image/svg+xml";
        if (p.endsWith(".png")) return "image/png";
        if (p.endsWith(".jpg") || p.endsWith(".jpeg")) return "image/jpeg";
        if (p.endsWith(".webp")) return "image/webp";
        if (p.endsWith(".woff2")) return "font/woff2";
        if (p.endsWith(".woff")) return "font/woff";
        if (p.endsWith(".ttf")) return "font/ttf";
        return "application/octet-stream";
    }

    @SuppressLint({"SetJavaScriptEnabled", "AddJavascriptInterface"})
    private WebView createWeb() {
        WebView w = new WebView(this);
        w.setBackgroundColor(Color.TRANSPARENT);
        WebSettings st = w.getSettings();
        st.setJavaScriptEnabled(true);
        st.setDomStorageEnabled(true);
        st.setMixedContentMode(WebSettings.MIXED_CONTENT_ALWAYS_ALLOW);
        w.addJavascriptInterface(new QmBridge(), "VisorQm");
        w.setWebViewClient(new WebViewClient() {
            @Override
            public WebResourceResponse shouldInterceptRequest(WebView v, WebResourceRequest r) {
                Uri u = r.getUrl();
                if (!"visor-qm.invalid".equals(u.getHost())) return null;
                String path = u.getPath();
                if (path == null || path.equals("/")) path = "/quickmenu.html";
                try {
                    InputStream in = getAssets().open("public" + path);
                    return new WebResourceResponse(mime(path), "utf-8", in);
                } catch (Exception e) {
                    return new WebResourceResponse("text/plain", "utf-8", 404, "Not Found", null, new ByteArrayInputStream(new byte[0]));
                }
            }
        });
        w.loadUrl("https://visor-qm.invalid/quickmenu.html");
        return w;
    }

    private void showMenu() {
        removeMenu();
        boolean fresh = web == null;
        if (fresh) web = createWeb();
        WindowManager.LayoutParams lp = new WindowManager.LayoutParams(
                WindowManager.LayoutParams.MATCH_PARENT, WindowManager.LayoutParams.MATCH_PARENT,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
                PixelFormat.TRANSLUCENT);
        try {
            ((WindowManager) getSystemService(Context.WINDOW_SERVICE)).addView(web, lp);
            menu = web;
            // La primera vez la propia pagina se abre al cargar; las siguientes hay que avisarla.
            if (!fresh) web.evaluateJavascript("window.__qmOpen&&window.__qmOpen()", null);
        } catch (Exception ignored) {
        }
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

    /** Consulta la sesion del PC: {estado, endedMs de la ultima, nombre del juego, app de Apollo} o null si no hay respuesta. */
    private Object[] sessionState() {
        String body = get("/api/v1/session");
        if (body == null) return null;
        try {
            JSONObject j = new JSONObject(body);
            JSONObject last = j.optJSONObject("last");
            JSONObject game = j.optJSONObject("game");
            String app = j.isNull("app") ? "" : j.optString("app", "");
            return new Object[] { j.optString("state", "idle"), last == null ? -1L : last.optLong("endedMs", -1), game == null ? "" : game.optString("name", ""), app };
        } catch (Exception e) {
            return null;
        }
    }

    /** Ultima vez que Artemis estuvo delante (ms): decide si al acabar la partida se trae el visor al frente. */
    private volatile long moonlightMs = 0;

    /**
     * Mientras la partida siga en marcha pregunta al PC por la sesion: cada 2 s con Artemis delante y cada 5 s fuera.
     * Con Artemis fuera (y el visor tambien) muestra la notificacion de partida en curso. Cuando ve una partida y despues el
     * reposo con un resumen nuevo, la partida acabo. Si el PC no responde unos 2,5 min seguidos, se rinde.
     */
    private void startWatch() {
        if (watching) return;
        watching = true;
        new Thread(() -> {
            try {
                long baseline = -2;
                boolean seen = false;
                boolean done = false;
                int fails = 0;
                while (true) {
                    boolean front = StreamingPlugin.isMoonlight(foreground);
                    if (front) moonlightMs = System.currentTimeMillis();
                    Object[] s = sessionState();
                    if (s == null) {
                        if (++fails >= 30) break;
                    } else {
                        fails = 0;
                        String state = (String) s[0];
                        long ended = (Long) s[1];
                        boolean active = "starting".equals(state) || "playing".equals(state);
                        if (baseline == -2) baseline = ended;
                        String app = (String) s[3];
                        if (!app.isEmpty()) getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE).edit().putString("r_app", app).apply();
                        if (active) {
                            seen = true;
                            final String name = (String) s[2];
                            final boolean show = !front && !getPackageName().equals(foreground);
                            ui.post(() -> { if (show) showOngoing(name); else hideOngoing(); });
                        } else {
                            if (seen && "idle".equals(state) && ended != baseline) { done = true; break; }
                            // Sin partida en marcha y Artemis fuera: no hay nada que vigilar.
                            if (!front) break;
                        }
                    }
                    try { Thread.sleep(front ? 2000 : 5000); } catch (InterruptedException ignored) { }
                }
                ui.post(this::hideOngoing);
                if (done) ui.post(this::sessionEnded);
            } finally {
                watching = false;
            }
        }, "visor-watch").start();
    }

    private void sessionEnded() {
        removeMenu();
        // Solo se trae el visor al frente si Artemis estaba delante o acababa de salir; en otra app, solo el aviso.
        boolean recent = StreamingPlugin.isMoonlight(foreground) || System.currentTimeMillis() - moonlightMs < 10000;
        if (!recent) { notifyEnded(); return; }
        returnToApp();
        // Android bloquea en silencio el arranque desde segundo plano en algunos dispositivos: si no funciono, aviso.
        ui.postDelayed(() -> {
            if (!getPackageName().equals(foreground)) notifyEnded();
        }, 1500);
    }

    private void showOngoing(String name) {
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        Intent open = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (nm == null || open == null) return;
        if (Build.VERSION.SDK_INT >= 26) {
            NotificationChannel ch = new NotificationChannel("partida", "Partida en curso", NotificationManager.IMPORTANCE_LOW);
            ch.setSound(null, null);
            nm.createNotificationChannel(ch);
        }
        open.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        int fl = PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE;
        PendingIntent pi = PendingIntent.getActivity(this, 1, open, fl);
        PendingIntent resume = PendingIntent.getActivity(this, 2, new Intent(this, ResumeActivity.class), fl);
        Notification.Builder b = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(this, "partida") : new Notification.Builder(this);
        b.setSmallIcon(getApplicationInfo().icon).setContentTitle(name.isEmpty() ? "Partida en curso" : name + " · Partida en curso")
                .setContentText("Toca para abrir el visor").setContentIntent(pi).setOngoing(true)
                .addAction(0, "Volver a la partida", resume);
        nm.notify(4102, b.build());
    }

    private void hideOngoing() {
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
        if (nm != null) nm.cancel(4102);
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

    private void toggleOverlay() {
        new Thread(() -> vibrate(post("/api/v1/overlay", null) != null), "visor-overlay").start();
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

package app.visorcapturas.movil;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.provider.Settings;
import android.text.TextUtils;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/** Puente con Moonlight (streaming del PC) y con el servicio que captura con la tecla de bajar volumen. */
@CapacitorPlugin(name = "Streaming")
public class StreamingPlugin extends Plugin {
    /** Artemis (fork de Moonlight para Apollo) primero, luego Moonlight; con sus variantes root y debug. */
    static final String[] MOONLIGHT = { "com.limelight.noir", "com.limelight.root.noir", "com.limelight.noirdebug", "com.limelight", "com.limelight.root" };
    static final String PREFS = "visor_streaming";

    static boolean isMoonlight(String pkg) {
        for (String p : MOONLIGHT) if (p.equals(pkg)) return true;
        return false;
    }

    private String installedMoonlight() {
        PackageManager pm = getContext().getPackageManager();
        for (String p : MOONLIGHT) {
            try {
                pm.getPackageInfo(p, 0);
                return p;
            } catch (PackageManager.NameNotFoundException ignored) {
            }
        }
        return null;
    }

    /** Tablet = ancho minimo de 600 dp (el mismo criterio que usa Android para sus recursos sw600dp). */
    @PluginMethod
    public void isTablet(PluginCall call) {
        JSObject r = new JSObject();
        r.put("tablet", getContext().getResources().getConfiguration().smallestScreenWidthDp >= 600);
        call.resolve(r);
    }

    @PluginMethod
    public void isMoonlightInstalled(PluginCall call) {
        JSObject r = new JSObject();
        r.put("installed", installedMoonlight() != null);
        call.resolve(r);
    }

    /** Abre directamente el stream de {app} en el PC {pc} (los nombres tal como los guarda Moonlight). */
    @PluginMethod
    public void stream(PluginCall call) {
        String pc = call.getString("pc", "");
        String app = call.getString("app", "Desktop");
        String pkg = installedMoonlight();
        if (pkg == null) {
            call.reject("No está instalado Artemis ni Moonlight");
            return;
        }
        // Moonlight busca el PC por nombre distinguiendo mayusculas; el que anuncia Sunshine es el que vale.
        String[] info = sunshineInfo();
        String uuid = info == null ? "" : info[1];
        if (info != null && !info[0].isEmpty()) pc = info[0];
        if ((pc == null || pc.isEmpty()) && uuid.isEmpty()) {
            call.reject("Falta el nombre del PC en Moonlight");
            return;
        }
        try {
            Intent i = new Intent();
            i.setComponent(new ComponentName(pkg, "com.limelight.ShortcutTrampoline"));
            if (!uuid.isEmpty()) i.putExtra("UUID", uuid);
            else i.putExtra("Name", pc);
            i.putExtra("AppName", app);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            getContext().startActivity(i);
            call.resolve();
        } catch (Exception e) {
            call.reject("No se pudo abrir Moonlight: " + e.getMessage());
        }
    }

    /** Nombre e identificador del PC tal como los anuncia Sunshine (puerto HTTP 47989); null si no responde. */
    private String[] sunshineInfo() {
        String base = getContext().getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString("base", "");
        String host = Uri.parse(base).getHost();
        if (host == null || host.isEmpty()) return null;
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL("http://" + host + ":47989/serverinfo?uniqueid=0123456789ABCDEF").openConnection();
            c.setConnectTimeout(2500);
            c.setReadTimeout(2500);
            if (c.getResponseCode() != 200) return null;
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[2048];
            try (InputStream in = c.getInputStream()) {
                for (int n; (n = in.read(buf)) > 0 && out.size() < 65536; ) out.write(buf, 0, n);
            }
            String xml = out.toString("UTF-8");
            return new String[] { xmlTag(xml, "hostname"), xmlTag(xml, "uniqueid") };
        } catch (Exception e) {
            return null;
        } finally {
            if (c != null) c.disconnect();
        }
    }

    private static String xmlTag(String xml, String tag) {
        int a = xml.indexOf("<" + tag + ">");
        int b = xml.indexOf("</" + tag + ">");
        return a < 0 || b < a ? "" : xml.substring(a + tag.length() + 2, b).trim();
    }

    /** Guarda dónde está el visor del PC para que el servicio de accesibilidad pueda pedir la captura. */
    @PluginMethod
    public void setConnection(PluginCall call) {
        SharedPreferences.Editor e = getContext().getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit();
        e.putString("base", call.getString("base", ""));
        e.putString("code", call.getString("code", ""));
        e.apply();
        call.resolve();
    }

    @PluginMethod
    public void openAccessibilitySettings(PluginCall call) {
        Intent i = new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS);
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        getContext().startActivity(i);
        call.resolve();
    }

    @PluginMethod
    public void isCaptureServiceEnabled(PluginCall call) {
        JSObject r = new JSObject();
        r.put("enabled", serviceEnabled(getContext()));
        call.resolve(r);
    }

    static boolean serviceEnabled(Context c) {
        String list = Settings.Secure.getString(c.getContentResolver(), Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES);
        if (list == null || list.isEmpty()) return false;
        ComponentName cn = new ComponentName(c, CaptureKeyService.class);
        TextUtils.SimpleStringSplitter sp = new TextUtils.SimpleStringSplitter(':');
        sp.setString(list);
        while (sp.hasNext()) {
            String s = sp.next();
            if (s.equalsIgnoreCase(cn.flattenToString()) || s.equalsIgnoreCase(cn.flattenToShortString())) return true;
        }
        return false;
    }
}

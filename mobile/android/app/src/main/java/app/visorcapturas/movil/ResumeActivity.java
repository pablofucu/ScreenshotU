package app.visorcapturas.movil;

import android.app.Activity;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Bundle;

/** Invisible: vuelve a abrir el stream de la partida en curso (boton de la notificacion) y termina. */
public class ResumeActivity extends Activity {
    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        SharedPreferences sp = getSharedPreferences(StreamingPlugin.PREFS, Context.MODE_PRIVATE);
        String pkg = sp.getString("r_pkg", "");
        if (!pkg.isEmpty()) {
            try {
                StreamingPlugin.launchStream(this, pkg, sp.getString("r_uuid", ""), sp.getString("r_pc", ""), sp.getString("r_app", "Desktop"));
            } catch (Exception ignored) {
            }
        }
        finish();
    }
}

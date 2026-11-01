package app.visorcapturas.movil;

import android.content.pm.ActivityInfo;
import android.os.Bundle;

import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(StreamingPlugin.class);
        super.onCreate(savedInstanceState);
        // Tablet (ancho mínimo ≥ 600 dp): la interfaz de escritorio es apaisada. El móvil no se toca.
        if (getResources().getConfiguration().smallestScreenWidthDp >= 600) {
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE);
        }
    }
}

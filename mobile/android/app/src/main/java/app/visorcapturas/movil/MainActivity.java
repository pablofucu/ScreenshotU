package app.visorcapturas.movil;

import android.os.Bundle;

import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(StreamingPlugin.class);
        super.onCreate(savedInstanceState);
    }
}

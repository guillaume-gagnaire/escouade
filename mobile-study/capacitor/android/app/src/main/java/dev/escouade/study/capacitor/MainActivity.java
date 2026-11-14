package dev.escouade.study.capacitor;

import android.os.Bundle;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        // Le plugin du Rust (ESC-445) est à l'app, pas à un paquet npm : il s'inscrit à la main.
        registerPlugin(RustCorePlugin.class);
        super.onCreate(savedInstanceState);
    }
}

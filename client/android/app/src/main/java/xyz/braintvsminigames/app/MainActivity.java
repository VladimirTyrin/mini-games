package xyz.braintvsminigames.app;

import android.content.Intent;
import android.os.Bundle;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        // The app's own plugins are registered before the bridge starts.
        registerPlugin(KeepAwakePlugin.class);
        // A replay file the app was started with: the bridge reads the intent as it starts.
        setIntent(IncomingReplay.rewrite(this, getIntent()));
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(IncomingReplay.rewrite(this, intent));
    }
}

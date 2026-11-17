package xyz.braintvsminigames.app;

import android.view.Window;
import android.view.WindowManager;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

/** Keeps the screen on during a match: Android's WebView has no Screen Wake Lock API. */
@CapacitorPlugin(name = "KeepAwake")
public class KeepAwakePlugin extends Plugin {
    @PluginMethod
    public void set(PluginCall call) {
        boolean keepScreenOn = Boolean.TRUE.equals(call.getBoolean("keepScreenOn", false));
        // Window flags may only be changed on the UI thread.
        getActivity().runOnUiThread(() -> {
            Window window = getActivity().getWindow();
            if (keepScreenOn) {
                window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
            } else {
                window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
            }
            call.resolve();
        });
    }
}

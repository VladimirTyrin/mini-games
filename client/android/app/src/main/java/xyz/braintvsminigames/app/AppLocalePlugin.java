package xyz.braintvsminigames.app;

import androidx.appcompat.app.AppCompatDelegate;
import androidx.core.os.LocaleListCompat;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

/**
 * The app's language as Android knows it, set from the language picked in the app: push
 * notifications are worded from the app's resources (values-ru), in this language rather than
 * the phone's. An empty tag follows the phone's language again. The activity handles the locale
 * change itself (configChanges), so the WebView is not restarted.
 */
@CapacitorPlugin(name = "AppLocale")
public class AppLocalePlugin extends Plugin {
    @PluginMethod
    public void set(PluginCall call) {
        String languageTag = call.getString("languageTag", "");
        getActivity().runOnUiThread(() -> {
            LocaleListCompat wanted = LocaleListCompat.forLanguageTags(languageTag);
            if (!wanted.equals(AppCompatDelegate.getApplicationLocales())) {
                AppCompatDelegate.setApplicationLocales(wanted);
            }
            call.resolve();
        });
    }
}

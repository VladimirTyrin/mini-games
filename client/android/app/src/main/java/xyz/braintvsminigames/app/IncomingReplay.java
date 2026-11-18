package xyz.braintvsminigames.app;

import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.util.Log;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * A replay file opened with the app ("Open with") or shared to it. The sending app grants access
 * to its file only for this intent, so the file is copied into the app's cache at once and the
 * intent becomes a link the web part reads it by: minigames-replay://open?file=NAME, NAME being the
 * copy in the cache's incoming-replay directory.
 */
final class IncomingReplay {
    static final String SCHEME = "minigames-replay";
    static final String DIRECTORY = "incoming-replay";
    private static final String TAG = "IncomingReplay";
    /** Twice the largest replay the app shares (4 MiB); anything bigger is not one. */
    private static final long MAX_BYTES = 8L * 1024 * 1024;

    private IncomingReplay() {}

    /** The intent to hand on: the link to the copied file, or `intent` itself when it brings none. */
    static Intent rewrite(Context context, Intent intent) {
        Uri source = sourceOf(intent);
        if (source == null) return intent;
        File copy = copyToCache(context, source);
        if (copy == null) return intent;
        Intent link = new Intent(Intent.ACTION_VIEW, Uri.parse(SCHEME + "://open?file=" + Uri.encode(copy.getName())));
        link.setFlags(intent.getFlags());
        return link;
    }

    private static Uri sourceOf(Intent intent) {
        if (intent == null) return null;
        Uri uri = null;
        if (Intent.ACTION_VIEW.equals(intent.getAction())) {
            uri = intent.getData();
        } else if (Intent.ACTION_SEND.equals(intent.getAction())) {
            uri = intent.getParcelableExtra(Intent.EXTRA_STREAM);
        }
        if (uri == null) return null;
        String scheme = uri.getScheme();
        // Friend links (https) are the web part's own business.
        return "content".equals(scheme) || "file".equals(scheme) ? uri : null;
    }

    private static File copyToCache(Context context, Uri source) {
        File directory = new File(context.getCacheDir(), DIRECTORY);
        // Only the latest one is kept: an opened replay is read once, at once.
        File[] old = directory.listFiles();
        if (old != null) for (File file : old) file.delete();
        if (!directory.isDirectory() && !directory.mkdirs()) return null;
        File copy = new File(directory, safeName(displayName(context, source)));
        try (InputStream in = context.getContentResolver().openInputStream(source);
             OutputStream out = new FileOutputStream(copy)) {
            if (in == null) return null;
            byte[] buffer = new byte[64 * 1024];
            long total = 0;
            for (int read; (read = in.read(buffer)) != -1; ) {
                total += read;
                if (total > MAX_BYTES) throw new IOException("too large for a replay");
                out.write(buffer, 0, read);
            }
            return copy;
        } catch (IOException | SecurityException error) {
            Log.w(TAG, "cannot read the incoming file", error);
            copy.delete();
            return null;
        }
    }

    private static String displayName(Context context, Uri source) {
        if ("content".equals(source.getScheme())) {
            try (Cursor cursor = context.getContentResolver()
                    .query(source, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
                if (cursor != null && cursor.moveToFirst()) {
                    String name = cursor.getString(0);
                    if (name != null && !name.isEmpty()) return name;
                }
            } catch (RuntimeException error) {
                Log.w(TAG, "no name for the incoming file", error);
            }
        }
        String last = source.getLastPathSegment();
        return last != null && !last.isEmpty() ? last : "replay.mgreplay";
    }

    /** A plain file name: the sender's may hold anything. */
    private static String safeName(String name) {
        String safe = name.replaceAll("[^A-Za-z0-9 ._()\\-]", "_");
        return safe.isEmpty() || safe.startsWith(".") ? "replay.mgreplay" : safe;
    }
}

package dev.horizon.android;

import android.app.Activity;
import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.net.Uri;
import android.os.Binder;
import android.os.Bundle;
import java.io.File;
import java.io.IOException;

/** World management stays inside the game sandbox. No arbitrary paths accepted. */
public final class VisGameStore extends ContentProvider {
    private static boolean active;
    private static void identifiers(String owner, String id) {
        if (owner == null || !owner.matches("[a-z0-9_]{3,24}") || id == null || !id.matches("[a-f0-9]{32}"))
            throw new IllegalArgumentException("Invalid instance");
    }
    private static File root(android.content.Context context, String storage) throws IOException {
        File home;
        if ("internal".equals(storage)) home = context.getFilesDir();
        else if ("external".equals(storage)) home = context.getExternalFilesDir(null);
        else throw new IllegalArgumentException("Invalid storage");
        if (home == null) throw new IOException("Storage unavailable");
        File root = new File(home.getCanonicalFile(), "vis-worlds");
        if (!root.getCanonicalFile().equals(root.getAbsoluteFile())) throw new IOException("Unsafe storage root");
        return root;
    }
    private static File instance(android.content.Context context, String storage, String owner, String id) throws IOException {
        File root = root(context, storage).getCanonicalFile();
        File path = new File(new File(root, owner), id);
        // Reject symlinked ancestors or instance roots; never delete outside this sandbox.
        if (!path.getCanonicalFile().equals(path.getAbsoluteFile())) throw new IOException("Unsafe instance path");
        return path;
    }
    private static String pref(String owner, String id) { return owner + "/" + id; }
    public static synchronized String begin(Activity activity, String owner, String id) throws IOException {
        identifiers(owner, id);
        if (active) throw new IOException("Game already running");
        String storage = activity.getSharedPreferences("vis-storage", 0).getString(pref(owner,id), "internal");
        File path = instance(activity, storage, owner,id);
        if (!path.isDirectory() && !path.mkdirs()) throw new IOException("Cannot create instance");
        active = true;
        return path.getAbsolutePath();
    }
    public static synchronized void end() { active = false; }
    @Override public Bundle call(String method, String arg, Bundle extras) {
        String[] callers = getContext().getPackageManager().getPackagesForUid(Binder.getCallingUid());
        boolean trusted = false;
        if (callers != null) for (String name: callers) if (name.equals("com.visstudios.nexo.mobile")) trusted = true;
        if (!trusted) throw new SecurityException("Nexo only");
        if ("diagnostics".equals(method)) return diagnostics();
        synchronized (VisGameStore.class) {
            Bundle result = new Bundle();
            try {
                if (active) throw new IOException("Close McRust before managing instances");
                String owner = extras.getString("owner"), id = extras.getString("instance");
                identifiers(owner,id);
                if (method.equals("prepare")) {
                    String storage = extras.getString("storage", "internal");
                    File chosen = instance(getContext(),storage,owner,id);
                    // A location change never hides or moves an existing world.
                    String previous = getContext().getSharedPreferences("vis-storage",0).getString(pref(owner,id),"internal");
                    if (!previous.equals(storage) && instance(getContext(),previous,owner,id).exists())
                        throw new IOException("Existing instance location cannot change");
                    if (!chosen.isDirectory() && !chosen.mkdirs()) throw new IOException("Storage unavailable");
                    if (!getContext().getSharedPreferences("vis-storage",0).edit().putString(pref(owner,id),storage).commit())
                        throw new IOException("Storage setting failed");
                } else if (method.equals("delete")) {
                    for (String storage: new String[]{"internal","external"}) {
                        // Do not claim deletion when a selected removable volume is unavailable.
                        File path = instance(getContext(),storage,owner,id);
                        File trash = new File(path.getParentFile(), ".deleted-" + id);
                        if (!trash.getCanonicalFile().equals(trash.getAbsoluteFile())) throw new IOException("Unsafe trash path");
                        if (trash.exists()) erase(trash);
                        if (path.exists()) {
                            if (!path.renameTo(trash)) throw new IOException("Cannot isolate instance");
                            erase(trash);
                        }
                    }
                    if (!getContext().getSharedPreferences("vis-storage",0).edit().remove(pref(owner,id)).commit())
                        throw new IOException("Cannot update instance storage");
                } else throw new IllegalArgumentException("Unsupported operation");
                result.putBoolean("ok",true);
            } catch (Exception error) { result.putBoolean("ok",false); result.putString("error","Cerrá McRust y comprobá que el almacenamiento esté disponible. No se quitó la instancia de Nexo."); }
            return result;
        }
    }
    private Bundle diagnostics() {
        Bundle result = new Bundle();
        StringBuilder report = new StringBuilder("VIS Nexo Rust Android\n");
        File startup = new File(getContext().getFilesDir(), "vis-startup-error");
        try (java.io.RandomAccessFile input = new java.io.RandomAccessFile(startup, "r")) {
            byte[] bytes = new byte[(int)Math.min(input.length(), 160)];
            input.readFully(bytes);
            report.append("Startup failure: ").append(startup.lastModified()).append('\n')
                .append("Etapa: ").append(new String(bytes, java.nio.charset.StandardCharsets.UTF_8)).append('\n');
        } catch (IOException ignored) {
            // A successful startup has no error file.
        }
        // Android 11+ exit history, without requiring the API on older devices.
        try {
            Object manager = android.content.Context.class.getMethod("getSystemService", String.class)
                .invoke(getContext(), "activity");
            java.util.List<?> exits = (java.util.List<?>) manager.getClass()
                .getMethod("getHistoricalProcessExitReasons", String.class, int.class, int.class)
                .invoke(manager, null, 0, 1);
            if (!exits.isEmpty()) {
                Object exit = exits.get(0);
                int reason = (Integer) exit.getClass().getMethod("getReason").invoke(exit);
                report.append("Android exit reason: ").append(reason)
                    .append("; status: ").append(exit.getClass().getMethod("getStatus").invoke(exit))
                    .append("; time: ").append(exit.getClass().getMethod("getTimestamp").invoke(exit)).append('\n');
                String cause;
                switch (reason) {
                    case 3: cause = "Presión de memoria"; break;
                    case 4: cause = "Excepción de la aplicación"; break;
                    case 5: cause = "Fallo nativo"; break;
                    case 6: cause = "La aplicación dejó de responder"; break;
                    case 2: cause = "Proceso terminado por señal"; break;
                    case 1: cause = "Salida del proceso (consultá el estado de salida)"; break;
                    case 7: cause = "Fallo al inicializar el proceso"; break;
                    case 9: cause = "Android terminó el proceso por uso excesivo de recursos"; break;
                    case 10: cause = "Cierre solicitado por el usuario o el sistema"; break;
                    case 12: cause = "Proceso asociado terminado"; break;
                    case 14: cause = "Android detectó un cambio en el estado del paquete"; break;
                    case 15: cause = "Android terminó un proceso congelado"; break;
                    case 16: cause = "El paquete del juego fue actualizado"; break;
                    default: cause = "Android no proporcionó un motivo concluyente";
                }
                if (cause != null) report.append(cause).append('\n');
            }
        } catch (ReflectiveOperationException | RuntimeException unavailable) {
            report.append("Android exit history unavailable\n");
        }
        File log = new File(getContext().getFilesDir(), "logs/latest.log");
        try {
            if (!log.getCanonicalFile().equals(log.getAbsoluteFile())) throw new IOException("Unsafe log");
            try (java.io.RandomAccessFile input = new java.io.RandomAccessFile(log, "r")) {
                int size = (int)Math.min(input.length(), 8192);
                input.seek(input.length() - size);
                byte[] bytes = new byte[size]; input.readFully(bytes);
                String tail = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
                for (String line : tail.split("\n")) {
                    if (line.contains("/ERROR]") || line.contains("/WARN]")) report.append(line).append('\n');
                }
            }
        } catch (IOException unavailable) { report.append("No engine log available\n"); }
        String text = report.toString();
        // Remove credentials and private paths before anything crosses into Nexo.
        text = text.replaceAll("(?i)Bearer\\s+[A-Za-z0-9._~+/-]+", "Bearer [OCULTO]")
            .replaceAll("(?i)(?:access_token|refresh_token|token|password|secret|authorization|ticket|device_code|api[_-]?key)\\s*[:=]\\s*[\\\"']?[^\\\"'\\s&,;}]+", "credencial=[OCULTA]")
            .replaceAll("(?:/data/|/storage/|/sdcard/|/home/|/Users/)[^\\s,;\\\"']+", "[RUTA PRIVADA]")
            .replaceAll("[\\w.+-]+@[\\w.-]+\\.[A-Za-z]{2,}", "[CORREO PRIVADO]");
        if (text.length() > 2000) text = text.substring(0, 200) + "\n" + text.substring(text.length()-1800);
        result.putString("report", text); result.putBoolean("ok", true);
        return result;
    }
    private static void erase(File file) throws IOException {
        // A symlink is unlinked, never traversed.
        if (file.getCanonicalFile().equals(file.getAbsoluteFile()) && file.isDirectory()) {
            File[] children = file.listFiles();
            if (children == null) throw new IOException("Cannot read instance");
            for (File child:children) erase(child);
        }
        if (!file.delete()) throw new IOException("Cannot remove instance data");
    }
    @Override public boolean onCreate(){return true;}
    @Override public Cursor query(Uri uri,String[] p,String s,String[] a,String o){throw new UnsupportedOperationException();}
    @Override public String getType(Uri uri){return null;}
    @Override public Uri insert(Uri uri,ContentValues values){throw new UnsupportedOperationException();}
    @Override public int delete(Uri uri,String s,String[] a){throw new UnsupportedOperationException();}
    @Override public int update(Uri uri,ContentValues v,String s,String[] a){throw new UnsupportedOperationException();}
}

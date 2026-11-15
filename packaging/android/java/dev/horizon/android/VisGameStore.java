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

package dev.horizon.android;

import android.app.Application;
import android.system.ErrnoException;
import android.system.Os;
import android.util.Log;

public final class VisGameApplication extends Application {
    @Override
    public void onCreate() {
        super.onCreate();
        try {
            // NativeActivity loads Rust afterwards. android-activity starts android_main
            // with Rust's default thread stack, which the game can exhaust during startup.
            Os.setenv("RUST_MIN_STACK", "8388608", true);
        } catch (ErrnoException error) {
            Log.e("VIS-McRust", "Could not configure the native thread stack", error);
        }
    }
}

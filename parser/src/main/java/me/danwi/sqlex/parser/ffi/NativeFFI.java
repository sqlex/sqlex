package me.danwi.sqlex.parser.ffi;

public class NativeFFI {

    static {
        FFIInvoker.loadNativeLibrary();
    }

    public static native String request(String req);
}
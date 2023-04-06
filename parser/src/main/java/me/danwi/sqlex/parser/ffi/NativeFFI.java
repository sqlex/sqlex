package me.danwi.sqlex.parser.ffi;

public class NativeFFI {
    static {
        System.out.println("当前加载器: " + NativeFFI.class.getClassLoader());
        System.out.println("原生库路径: " + NativeLib.dylibFilePath);
        System.load(NativeLib.dylibFilePath);
    }

    public static native String request(String req);
}
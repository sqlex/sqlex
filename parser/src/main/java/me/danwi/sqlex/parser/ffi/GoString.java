package me.danwi.sqlex.parser.ffi;

import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;
import org.jetbrains.annotations.Nullable;

import java.nio.charset.StandardCharsets;

@Structure.FieldOrder({"address", "length"})
public class GoString extends Structure {
    public static class ByValue extends GoString implements Structure.ByValue {
        public ByValue(String str) {
            byte[] strData = str.getBytes(StandardCharsets.UTF_8);
            Pointer ptr = new Memory(strData.length);
            ptr.write(0, strData, 0, strData.length);
            this.address = ptr;
            this.length = str.getBytes(StandardCharsets.UTF_8).length;
        }

        public @Nullable
        String getString() {
            if (this.address == null)
                return null;
            return new String(this.address.getByteArray(0, this.length), StandardCharsets.UTF_8);
        }

        public void free() {
            if (this.address != null)
                Native.free(Pointer.nativeValue(this.address));
        }
    }

    public Pointer address;
    public int length;
}

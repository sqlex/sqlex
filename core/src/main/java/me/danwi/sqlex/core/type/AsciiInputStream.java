package me.danwi.sqlex.core.type;

import java.io.IOException;
import java.io.InputStream;

/**
 * 此类型会使用 {@link java.sql.PreparedStatement#setAsciiStream(int, InputStream)} 设置参数
 *
 * @author wjy
 */
public class AsciiInputStream extends InputStream {

    private InputStream inputStream;

    public AsciiInputStream(InputStream inputStream) {
        this.inputStream = inputStream;
    }

    @Override
    public int read() throws IOException {
        return this.inputStream.read();
    }
}

package me.danwi.sqlex.core.type;

import java.io.IOException;
import java.io.InputStream;

/**
 * 此类型会使用 {@link java.sql.PreparedStatement#setBlob(int, InputStream)} 设置参数
 *
 * @author wjy
 */
public class BlobInputStream extends InputStream {
    private InputStream inputStream;

    public BlobInputStream(InputStream inputStream) {
        this.inputStream = inputStream;
    }

    @Override
    public int read() throws IOException {
        return this.inputStream.read();
    }
}

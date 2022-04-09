package me.danwi.sqlex.core.type;

import java.io.IOException;
import java.io.InputStream;

/**
 * @author wjy
 * @date 2022/4/9
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

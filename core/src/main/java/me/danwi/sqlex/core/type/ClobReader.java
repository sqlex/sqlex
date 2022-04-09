package me.danwi.sqlex.core.type;

import java.io.IOException;
import java.io.Reader;

/**
 * @author wjy
 * @date 2022/4/9
 */
public class ClobReader extends Reader {

    private Reader reader;

    public ClobReader(Reader reader) {
        this.reader = reader;
    }

    @Override
    public int read(char[] cbuf, int off, int len) throws IOException {
        return this.reader.read(cbuf, off, len);
    }

    @Override
    public void close() throws IOException {
        this.reader.close();
    }
}

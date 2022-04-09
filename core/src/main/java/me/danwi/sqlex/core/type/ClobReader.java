package me.danwi.sqlex.core.type;

import java.io.IOException;
import java.io.Reader;

/**
 * 此类型会使用 {@link java.sql.PreparedStatement#setClob(int, Reader)} 设置参数
 *
 * @author wjy
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

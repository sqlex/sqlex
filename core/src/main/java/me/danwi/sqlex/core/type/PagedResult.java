package me.danwi.sqlex.core.type;


import java.util.Collections;
import java.util.List;

public class PagedResult<T> {
    private long pageSize;
    private long pageNo;
    private long total;
    private List<T> data;

    public PagedResult(long pageSize, long pageNo, long total, List<T> data) {
        this.pageSize = pageSize;
        this.pageNo = pageNo;
        this.total = total;
        this.data = data == null ? Collections.emptyList() : data;
    }

    public long getPageSize() {
        return pageSize;
    }

    public long getPageNo() {
        return pageNo;
    }

    public long getTotal() {
        return total;
    }

    public List<T> getData() {
        return data;
    }
}

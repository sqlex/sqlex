package me.danwi.sqlex.core.query;

public class TableDelete extends WhereBuilder<TableDelete> {
    /**
     * 执行删除
     *
     * @return 删除的行数
     */
    long execute() {
        throw new UnsupportedOperationException();
    }
}

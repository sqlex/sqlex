package me.danwi.sqlex.core.query;

public class InsertOption {
    /**
     * 没有选项
     */
    public static int NONE_OPTIONS = 0;
    /**
     * 如果值为NULL,则表示该值为设置
     */
    public static int NULL_IS_NONE = 1;
    /**
     * 如果存在key冲突,则更新
     */
    public static int INSERT_OR_UPDATE = 1 << 1;
}

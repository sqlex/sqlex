package me.danwi.sqlex.common;

public class ParameterTypes {
    //预支持的类型
    public static final String[] PreSupportedTypes = {
            //基本类型
            "java.lang.Boolean",
            "java.lang.Byte",
            "java.lang.Short",
            "java.lang.Integer",
            "java.lang.Long",
            "java.lang.Float",
            "java.lang.Double",
            "java.lang.Character",
            "java.lang.String",
            "java.math.BigDecimal",
            "java.math.BigInteger",
            //字符 / 二进制数据
            "java.lang.Byte[]",
            "java.sql.Blob",
            "java.io.InputStream",
            "java.io.Reader",
            //时间
            "java.sql.Date",
            "java.sql.Time",
            "java.sql.Timestamp",
            "java.util.Date",
            "java.time.LocalDate",
            "java.time.LocalTime",
            "java.time.LocalDateTime",
            "java.time.OffsetTime",
            "java.time.OffsetDateTime",
            "java.time.ZonedDateTime",
            "java.time.Instant",
    };
}

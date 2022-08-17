package me.danwi.sqlex.spring;

import me.danwi.sqlex.core.ExceptionTranslator;
import me.danwi.sqlex.core.exception.SqlExUndeclaredException;
import org.springframework.dao.DataAccessException;
import org.springframework.jdbc.support.SQLErrorCodeSQLExceptionTranslator;
import org.springframework.jdbc.support.SQLExceptionTranslator;

import javax.sql.DataSource;
import java.sql.SQLException;

/**
 * 由spring框架支持都异常翻译
 *
 * <p>{@link SQLException}将通过转换成spring标准的{@link org.springframework.dao.DataAccessException}
 *
 * <p>{@link RuntimeException}分类异常保持不变
 *
 * <p>其他的Checked异常,将全部转换成{@link SqlExUndeclaredException}
 */
public class SpringExceptionTranslator implements ExceptionTranslator {
    private final SQLExceptionTranslator springTranslator;

    SpringExceptionTranslator(DataSource dataSource) {
        springTranslator = new SQLErrorCodeSQLExceptionTranslator(dataSource);
    }

    @Override
    public RuntimeException translate(Exception ex) {
        if (ex instanceof SQLException) {
            //TODO: 异常SQL的补充
            DataAccessException dataAccessException = springTranslator.translate("SqlEx", "", (SQLException) ex);
            //交由spring的异常翻译来处理,如果找不到匹配的DataAccessException,此处可能为空,交由下方处理
            if (dataAccessException != null)
                return dataAccessException;
        }
        if (ex instanceof RuntimeException)
            return (RuntimeException) ex;
        else
            return new SqlExUndeclaredException(ex);
    }
}

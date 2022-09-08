package me.danwi.sqlex.common;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class SQLUtilsTest {
    @Test
    void replaceDatabaseName() {
        Map<String, String> mapping = new HashMap<>();
        mapping.put("orders", "orders_test_db");
        mapping.put("users", "users_test_db");

        String sql = SQLUtils.replaceDatabaseName("select * from test left join orders.test on test.id=orders.test.id", mapping);
        assertEquals("select * from test left join orders_test_db.test on test.id=orders_test_db.test.id", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test left join orders.test on users.test.id = orders.test.id", mapping);
        assertEquals("select * from users_test_db.test left join orders_test_db.test on users_test_db.test.id = orders_test_db.test.id", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test t1 left join orders.test t2 on t1.id = t2.id", mapping);
        assertEquals("select * from users_test_db.test t1 left join orders_test_db.test t2 on t1.id = t2.id", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test left join orders.test on users.test.id = orders.test.id where users.test.id = 1", mapping);
        assertEquals("select * from users_test_db.test left join orders_test_db.test on users_test_db.test.id = orders_test_db.test.id where users_test_db.test.id = 1", sql);

        sql = SQLUtils.replaceDatabaseName("select users.test.id from users.test", mapping);
        assertEquals("select users_test_db.test.id from users_test_db.test", sql);

        sql = SQLUtils.replaceDatabaseName("select * from test", mapping);
        assertEquals("select * from test", sql);

        sql = SQLUtils.replaceDatabaseName("select * from test limit ? offset ?", mapping);
        assertEquals("select * from test limit ? offset ?", sql);
    }

    @Test
    void splitStatements() {
        List<String> sqls = SQLUtils.splitStatements("select 1; select 2; select 3");
        assertArrayEquals(new String[]{"select 1", "select 2", "select 3"}, sqls.toArray());

        sqls = SQLUtils.splitStatements("select 1; select 2; select 3;");
        assertArrayEquals(new String[]{"select 1", "select 2", "select 3"}, sqls.toArray());

        sqls = SQLUtils.splitStatements("select 1; select 2  ; select 3  ");
        assertArrayEquals(new String[]{"select 1", "select 2", "select 3"}, sqls.toArray());

        sqls = SQLUtils.splitStatements("select 1; select 2  ; select 3;show tables; show databases;");
        assertArrayEquals(new String[]{"select 1", "select 2", "select 3", "show tables", "show databases"}, sqls.toArray());

        sqls = SQLUtils.splitStatements("insert into test values(1,2,3); DELETE from person where 1=1;select * from person");
        assertArrayEquals(new String[]{"insert into test values(1,2,3)", "DELETE from person where 1=1", "select * from person"}, sqls.toArray());

        sqls = SQLUtils.splitStatements("insert into test values(1,2,3);\n alter table person add column age integer not null\n;");
        assertArrayEquals(new String[]{"insert into test values(1,2,3)", "alter table person add column age integer not null"}, sqls.toArray());
    }
}

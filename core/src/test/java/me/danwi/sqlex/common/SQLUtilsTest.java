package me.danwi.sqlex.common;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashMap;
import java.util.Map;

public class SQLUtilsTest {
    @Test
    void replaceDatabaseName() {
        Map<String, String> mapping = new HashMap<>();
        mapping.put("orders", "orders_test_db");
        mapping.put("users", "users_test_db");

        String sql = SQLUtils.replaceDatabaseName("select * from test left join orders.test", mapping);
        assertEquals("SELECT *\nFROM test\n\tLEFT JOIN orders_test_db.test", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test left join orders.test", mapping);
        assertEquals("SELECT *\nFROM users_test_db.test\n\tLEFT JOIN orders_test_db.test", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test t1 left join orders.test t2 on t1.id = t2.id", mapping);
        assertEquals("SELECT *\nFROM users_test_db.test t1\n\tLEFT JOIN orders_test_db.test t2 ON t1.id = t2.id", sql);

        sql = SQLUtils.replaceDatabaseName("select * from users.test left join orders.test on users.test.id = orders.test.id where users.test.id = 1", mapping);
        assertEquals("SELECT *\nFROM users_test_db.test\n\tLEFT JOIN orders_test_db.test ON users_test_db.test.id = orders_test_db.test.id\nWHERE users_test_db.test.id = 1", sql);

        sql = SQLUtils.replaceDatabaseName("select users.test.id from users.test", mapping);
        assertEquals("SELECT users_test_db.test.id\nFROM users_test_db.test", sql);

        sql = SQLUtils.replaceDatabaseName("select * from test", mapping);
        assertEquals("select * from test", sql);
    }
}

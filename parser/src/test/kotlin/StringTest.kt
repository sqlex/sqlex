import me.danwi.sqlex.parser.util.NamedParameter
import me.danwi.sqlex.parser.util.namedParameterSQL
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*

class StringTest {
    @Test
    fun noParameter() {
        val sql = "select * from person"
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals(sql, namedParameterSQL.sql)
        assertEquals(0, namedParameterSQL.parameters.size)
    }

    @Test
    fun haveParameters() {
        val sql = "select * from person where age > :age and name = :name and age > :age"
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals("select * from person where age > ? and name = ? and age > ?", namedParameterSQL.sql)
        assertEquals(
            listOf(
                NamedParameter("age", 33),
                NamedParameter("name", 46),
                NamedParameter("age", 58)
            ),
            namedParameterSQL.parameters
        )
    }

    @Test
    fun haveParametersWithChinese() {
        val sql = "select * from person where chinese = '中国人' and age > :age and name = :name"
        assertEquals('中', sql[38])
        assertEquals('国', sql[39])
        assertEquals('人', sql[40])
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals("select * from person where chinese = '中国人' and age > ? and name = ?", namedParameterSQL.sql)
        assertEquals(
            listOf(
                NamedParameter("age", 53),
                NamedParameter("name", 66)
            ),
            namedParameterSQL.parameters
        )
    }

    @Test
    fun parameterInQuote() {
        val sql = "select * from person where name = '123 :name'"
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals(sql, namedParameterSQL.sql)
        assertEquals(0, namedParameterSQL.parameters.size)
    }

    @Test
    fun parameterInDoubleQuote() {
        val sql = "select * from person where name = \"123 :name\""
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals(sql, namedParameterSQL.sql)
        assertEquals(0, namedParameterSQL.parameters.size)
    }

    @Test
    fun parameterInComment() {
        val sql = """
            -- :name
            select * from person;
        """.trimIndent()
        val namedParameterSQL = sql.namedParameterSQL
        assertEquals(sql, namedParameterSQL.sql)
        assertEquals(0, namedParameterSQL.parameters.size)
    }
}
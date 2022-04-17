import me.danwi.sqlex.parser.Session
import me.danwi.sqlex.parser.StatementType
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

class SessionTest {
    @Test
    fun createAndClose() {
        val session = Session("createAndClose_test")
        session.close()
    }

    @Test
    fun createTable() {
        val session = Session("createTable_test")
        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        assertArrayEquals(arrayOf("person"), session.allTables)
        //language=MySQL
        session.execute(
            """
            create table department(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column departmentID int not null
        """.trimIndent()
        )
        assertArrayEquals(arrayOf("department", "person"), session.allTables)
        session.close()
    }

    @Test
    fun ddl() {
        val session = Session("ddl_test")

        //language=MySQL
        session.executeScript(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null 
            );
            
            create table department(
                id int auto_increment primary key,
                name varchar(255) not null 
            );
            
            alter table person add column departmentID int not null;
        """.trimIndent()
        )

        assertEquals(
            """
            /* department */
            CREATE TABLE `department` (
              `id` int(11) NOT NULL AUTO_INCREMENT,
              `name` varchar(255) NOT NULL,
              PRIMARY KEY (`id`) /*T![clustered_index] CLUSTERED */
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

            /* person */
            CREATE TABLE `person` (
              `id` int(11) NOT NULL AUTO_INCREMENT,
              `name` varchar(255) NOT NULL,
              `departmentID` int(11) NOT NULL,
              PRIMARY KEY (`id`) /*T![clustered_index] CLUSTERED */
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
        """.trimIndent(), session.DDL
        )

        session.close()
    }

    @Test
    fun getFields() {
        var session = Session("getFields_test")
        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            create table department(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column departmentID int not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session,需要处理一下golang那边的BUG
        session = Session("getFields_test")

        var fields = session.getFields("select * from person")
        assertEquals(3, fields.size)

        assertEquals("id", fields[0].name)
        assertEquals("int", fields[0].dbType)
        assertEquals(11, fields[0].length)

        assertEquals("name", fields[1].name)
        assertEquals("varchar", fields[1].dbType)
        assertEquals(255, fields[1].length)

        assertEquals("departmentID", fields[2].name)
        assertEquals("int", fields[2].dbType)
        assertEquals(11, fields[2].length)

        //language=MySQL
        fields = session.getFields(
            """
            select department.name as name, count(1) as amount from person
                left join department on person.departmentID=department.id
            group by department.name
            order by amount
        """.trimIndent()
        )
        assertEquals(2, fields.size)

        assertEquals("name", fields[0].name)
        assertEquals("varchar", fields[0].dbType)
        assertEquals(255, fields[0].length)

        assertEquals("amount", fields[1].name)
        assertEquals("bigint", fields[1].dbType)
        assertEquals(21, fields[1].length)

        session.close()
    }

    @Test
    fun getStatementType() {
        val session = Session("getInExprPositions_test")

        var sql = "select * from person"
        assertEquals(StatementType.Select, session.getStatementInfo(sql).type)

        sql = """
            select * from person
            union all
            select * from person
        """.trimIndent()
        assertEquals(StatementType.Select, session.getStatementInfo(sql).type)

        sql = """
            with recursive temp as (
                select * from person
                union all
                select * from person
            )
            select * from temp
        """.trimIndent()
        assertEquals(StatementType.Select, session.getStatementInfo(sql).type)

        sql = "insert into person values(1,2,3,4)"
        assertEquals(StatementType.Insert, session.getStatementInfo(sql).type)

        sql = "update person set age = 30 where id =1"
        assertEquals(StatementType.Update, session.getStatementInfo(sql).type)

        sql = "delete from person where id = 1"
        assertEquals(StatementType.Delete, session.getStatementInfo(sql).type)

        sql = "show tables"
        assertEquals(StatementType.Other, session.getStatementInfo(sql).type)

        sql = "drop table person"
        assertEquals(StatementType.Other, session.getStatementInfo(sql).type)

        sql = "drop database person"
        assertEquals(StatementType.Other, session.getStatementInfo(sql).type)

        sql = "create database test"
        assertEquals(StatementType.Other, session.getStatementInfo(sql).type)

        session.close()
    }

    @Test
    fun getInExprPositions() {
        val session = Session("getInExprPositions_test")

        var sql = "select * from person where name in (?)"

        var positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("name in (?)", sql.substring(positions[0].start, positions[0].end))

        sql = "select * from person where chinese = '中国人' and name in (?)"
        positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("name in (?)", sql.substring(positions[0].start, positions[0].end))

        sql = "select * from person where name in (select name from person where name = 'nobody')"
        positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(0, positions.size)

        sql = "select * from person where name in ('alice', 'bob', 'candy')"
        positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(0, positions.size)

        sql = "select * from person where chinese = '中国人' and name in (?) and age not in (?)"
        positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(2, positions.size)

        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("name in (?)", sql.substring(positions[0].start, positions[0].end))

        assertTrue(positions[1].not)
        assertEquals('?', sql[positions[1].marker])
        assertEquals("age not in (?)", sql.substring(positions[1].start, positions[1].end))

        sql = """
            select * from person
                where chinese = '中国人'
                  and name in (?) 
                  and age not in (?)
        """.trimIndent()
        positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(2, positions.size)

        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("name in (?)", sql.substring(positions[0].start, positions[0].end))

        assertTrue(positions[1].not)
        assertEquals('?', sql[positions[1].marker])
        assertEquals("age not in (?)", sql.substring(positions[1].start, positions[1].end))

        session.close()
    }

    @Test
    fun getLimitPositions() {
        var session = Session("getLimitPositions_test")

        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            create table department(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column departmentID int not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session
        session = Session("getLimitPositions_test")

        var sql = "select * from person limit ?"

        var positions = session.getStatementInfo(sql).limitPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].hasOffset)
        assertEquals('?', sql[positions[0].count])

        sql = "select * from person where name = '中国人' limit ?"
        positions = session.getStatementInfo(sql).limitPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].hasOffset)
        assertEquals('?', sql[positions[0].count])

        sql = "select * from person where name in (select name from person limit ?) limit ?"
        positions = session.getStatementInfo(sql).limitPositions
        assertEquals(2, positions.size)
        assertFalse(positions[0].hasOffset)
        assertEquals('?', sql[positions[0].count])

        assertFalse(positions[1].hasOffset)
        assertEquals('?', sql[positions[1].count])

        sql = "select * from person where name = '中国人' limit ?, ?"
        positions = session.getStatementInfo(sql).limitPositions
        assertEquals(1, positions.size)

        assertTrue(positions[0].hasOffset)
        assertEquals('?', sql[positions[0].count])
        assertEquals('?', sql[positions[0].offset])

        sql = "select * from person where name = '中国人' limit ? offset ?"
        positions = session.getStatementInfo(sql).limitPositions
        assertEquals(1, positions.size)

        assertTrue(positions[0].hasOffset)
        assertEquals('?', sql[positions[0].count])
        assertEquals('?', sql[positions[0].offset])

        session.close()
    }

    @Test
    fun getLimitRows() {
        var session = Session("getLimitRows_test")

        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            create table department(
                id int auto_increment primary key,
                name varchar(255) not null 
            )
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column departmentID int not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session
        session = Session("getLimitRows_test")

        var sql = "select * from person limit 1"
        var info = session.getStatementInfo(sql)
        assertTrue(info.hasLimit)
        assertEquals(1, info.limitRows.toInt())

        sql = "select * from person limit 0"
        info = session.getStatementInfo(sql)
        assertFalse(info.hasLimit)

        sql = "select * from person limit 100,1"
        info = session.getStatementInfo(sql)
        assertTrue(info.hasLimit)
        assertEquals(1, info.limitRows.toInt())

        sql = "select * from person limit 10 offset 100"
        info = session.getStatementInfo(sql)
        assertTrue(info.hasLimit)
        assertEquals(10, info.limitRows.toInt())

        sql = "select * from person limit ?"
        info = session.getStatementInfo(sql)
        assertFalse(info.hasLimit)

        sql = "select * from person limit ?, 1"
        info = session.getStatementInfo(sql)
        assertTrue(info.hasLimit)
        assertEquals(1, info.limitRows.toInt())

        sql = "select * from person limit 100, ?"
        info = session.getStatementInfo(sql)
        assertFalse(info.hasLimit)
    }
}
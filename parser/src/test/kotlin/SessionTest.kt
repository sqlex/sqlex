import me.danwi.sqlex.parser.Session
import me.danwi.sqlex.parser.StatementType
import me.danwi.sqlex.parser.exception.SqlExFFIInvokeException
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
    fun allTables() {
        var session = Session("allTables_test")
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
        //language=MySQL
        session.execute(
            """
            alter table person add column age int unsigned not null
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column status char(255) charset binary not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session,需要处理一下golang那边的BUG
        session = Session("allTables_test")

        assertEquals(session.allTables.size, 2)
        assertArrayEquals(session.allTables, arrayOf("department", "person"))
    }

    @Test
    fun getColumns() {
        var session = Session("getColumns_test")
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
        //language=MySQL
        session.execute(
            """
            alter table person add column age int unsigned not null
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column status char(255) charset binary not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session,需要处理一下golang那边的BUG
        session = Session("getColumns_test")

        var fields = session.getColumns("person")
        assertEquals(5, fields.size)

        assertEquals("id", fields[0].name)
        assertEquals("int", fields[0].dbType)
        assertEquals(11, fields[0].length)
        assertFalse(fields[0].unsigned)
        assertTrue(fields[0].isPrimaryKey)
        assertFalse(fields[0].isPartOfMultipleKey)
        assertTrue(fields[0].isAutoIncrement)
        assertFalse(fields[0].isUnique)
        assertTrue(fields[0].notNull)
        assertTrue(fields[0].hasDefaultValue)

        assertEquals("name", fields[1].name)
        assertEquals("varchar", fields[1].dbType)
        assertEquals(255, fields[1].length)
        assertFalse(fields[1].binary)
        assertFalse(fields[1].isPrimaryKey)
        assertFalse(fields[1].isPartOfMultipleKey)
        assertFalse(fields[1].isAutoIncrement)
        assertFalse(fields[1].isUnique)
        assertTrue(fields[1].notNull)
        assertFalse(fields[1].hasDefaultValue)

        assertEquals("departmentID", fields[2].name)
        assertEquals("int", fields[2].dbType)
        assertEquals(11, fields[2].length)
        assertFalse(fields[2].unsigned)
        assertFalse(fields[1].isPrimaryKey)
        assertFalse(fields[1].isPartOfMultipleKey)
        assertFalse(fields[1].isAutoIncrement)
        assertFalse(fields[1].isUnique)
        assertTrue(fields[1].notNull)
        assertFalse(fields[1].hasDefaultValue)

        assertEquals("age", fields[3].name)
        assertEquals("int", fields[3].dbType)
        assertEquals(10, fields[3].length)
        assertTrue(fields[3].unsigned)
        assertFalse(fields[1].isPrimaryKey)
        assertFalse(fields[1].isPartOfMultipleKey)
        assertFalse(fields[1].isAutoIncrement)
        assertFalse(fields[1].isUnique)
        assertTrue(fields[1].notNull)
        assertFalse(fields[1].hasDefaultValue)

        assertEquals("status", fields[4].name)
        assertEquals("binary", fields[4].dbType)
        assertEquals(255, fields[4].length)
        assertTrue(fields[4].binary)
        assertFalse(fields[1].isPrimaryKey)
        assertFalse(fields[1].isPartOfMultipleKey)
        assertFalse(fields[1].isAutoIncrement)
        assertFalse(fields[1].isUnique)
        assertTrue(fields[1].notNull)
        assertFalse(fields[1].hasDefaultValue)

        //department表
        fields = session.getColumns("department")
        assertEquals(2, fields.size)

        assertEquals("id", fields[0].name)
        assertEquals("int", fields[0].dbType)
        assertEquals(11, fields[0].length)
        assertFalse(fields[0].unsigned)
        assertTrue(fields[0].isPrimaryKey)
        assertFalse(fields[0].isPartOfMultipleKey)
        assertTrue(fields[0].isAutoIncrement)
        assertFalse(fields[0].isUnique)
        assertTrue(fields[0].notNull)
        assertTrue(fields[0].hasDefaultValue)

        assertEquals("name", fields[1].name)
        assertEquals("varchar", fields[1].dbType)
        assertEquals(255, fields[1].length)
        assertFalse(fields[1].binary)
        assertFalse(fields[1].isPrimaryKey)
        assertFalse(fields[1].isPartOfMultipleKey)
        assertFalse(fields[1].isAutoIncrement)
        assertFalse(fields[1].isUnique)
        assertTrue(fields[1].notNull)
        assertFalse(fields[1].hasDefaultValue)
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
        //language=MySQL
        session.execute(
            """
            alter table person add column age int unsigned not null
        """.trimIndent()
        )
        //language=MySQL
        session.execute(
            """
            alter table person add column status char(255) charset binary not null
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session,需要处理一下golang那边的BUG
        session = Session("getFields_test")

        var fields = session.getPlanInfo("select * from person").fields
        assertEquals(5, fields.size)

        assertEquals("id", fields[0].name)
        assertEquals("int", fields[0].dbType)
        assertEquals(11, fields[0].length)
        assertFalse(fields[0].unsigned)

        assertEquals("name", fields[1].name)
        assertEquals("varchar", fields[1].dbType)
        assertEquals(255, fields[1].length)
        assertFalse(fields[1].binary)

        assertEquals("departmentID", fields[2].name)
        assertEquals("int", fields[2].dbType)
        assertEquals(11, fields[2].length)
        assertFalse(fields[2].unsigned)

        assertEquals("age", fields[3].name)
        assertEquals("int", fields[3].dbType)
        assertEquals(10, fields[3].length)
        assertTrue(fields[3].unsigned)

        assertEquals("status", fields[4].name)
        assertEquals("binary", fields[4].dbType)
        assertEquals(255, fields[4].length)
        assertTrue(fields[4].binary)


        //language=MySQL
        fields = session.getPlanInfo(
            """
            select department.name as name, count(1) as amount from person
                left join department on person.departmentID=department.id
            group by department.name
            order by amount
        """.trimIndent()
        ).fields
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
    fun valid() {
        var session = Session("valid_test")
        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null,
                age int unsigned not null
            )
        """.trimIndent()
        )
        session.close()

        //TODO:重新开session,需要处理一下golang那边的BUG
        session = Session("valid_test")
        //insert语句
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("insert into person values('')") }
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("insert into person(names) values('')") }
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("insert into person(names) values(?)") }
        assertDoesNotThrow { session.getPlanInfo("insert into person values(null, '', '1')") }
        assertDoesNotThrow { session.getPlanInfo("insert into person values(null, '', 1)") }
        assertDoesNotThrow { session.getPlanInfo("insert into person(name) values('')") }
        assertDoesNotThrow { session.getPlanInfo("insert into person(name) values(?)") }
        //update语句
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("update person set names = ''") }
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("update person set names = '' where id = 1") }
        assertDoesNotThrow { session.getPlanInfo("update person set name = '' where id = ''") }
        //delete语句
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("delete from person where names = ''") }
        assertThrowsExactly(SqlExFFIInvokeException::class.java) { session.getPlanInfo("delete from person where names = ?") }
        assertDoesNotThrow { session.getPlanInfo("delete from person") }
        assertDoesNotThrow { session.getPlanInfo("delete from person where name = ''") }
        assertDoesNotThrow { session.getPlanInfo("delete from person where name = ? ") }

        session.close()
    }

    @Test
    fun getMaxOneRow() {
        var session = Session("getMaxOneRow_test")

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

        //重开session
        session = Session("getMaxOneRow_test")

        assertTrue(session.getPlanInfo("select 1").maxOneRow)
        assertTrue(session.getPlanInfo("select * from person limit 1").maxOneRow)
        assertTrue(session.getPlanInfo("select * from person where id = ?").maxOneRow)
        assertTrue(session.getPlanInfo("select count(*) from person where name like ?").maxOneRow)
        assertTrue(session.getPlanInfo("select sum(id) from person").maxOneRow)

        assertFalse(session.getPlanInfo("select * from person").maxOneRow)
        assertFalse(session.getPlanInfo("select name, count(*) from person group by name").maxOneRow)

    }

    @Test
    fun getStatementType() {
        var session = Session("getStatementType_test")

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

        //重开session
        session = Session("getStatementType_test")

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
        var session = Session("getInExprPositions_test")

        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null,
                age int not null
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

        //重开session
        session = Session("getInExprPositions_test")

        var sql = "select * from person where name in (?)"

        var positions = session.getStatementInfo(sql).inExprPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("name in (?)", sql.substring(positions[0].start, positions[0].end))

        sql = "select * from person where name = '中国人' and name in (?)"
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

        sql = "select * from person where name = '中国人' and name in (?) and age not in (?)"
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
                where name = '中国人'
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
    fun getIsNullExprPositions() {
        var session = Session("getIsNullExprPositions_test")

        //language=MySQL
        session.execute(
            """
            create table person(
                id int auto_increment primary key,
                name varchar(255) not null,
                age int not null
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

        //重开session
        session = Session("getIsNullExprPositions_test")

        var sql = "select * from person where (? is null or name = ?)"

        var positions = session.getStatementInfo(sql).isNullExprPositions
        assertEquals(1, positions.size)
        assertFalse(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("? is null", sql.substring(positions[0].start, positions[0].end))

        sql = "select * from person where (? is not null or name = ?)"

        positions = session.getStatementInfo(sql).isNullExprPositions
        assertEquals(1, positions.size)
        assertTrue(positions[0].not)
        assertEquals('?', sql[positions[0].marker])
        assertEquals("? is not null", sql.substring(positions[0].start, positions[0].end))

        sql = "select * from person where (name is null or name = ?)"

        positions = session.getStatementInfo(sql).isNullExprPositions
        assertEquals(0, positions.size)

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
        assertTrue(info.hasLimit)
        assertEquals(0, info.limitRows.toInt())

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

        //测union中的情形
        sql = """
            select * from person
            union
            select * from person
            limit 1
        """.trimIndent()
        info = session.getStatementInfo(sql)
        assertTrue(info.hasLimit)
        assertEquals(1, info.limitRows.toInt())

        sql = """
            (select * from person limit 1)
            union
            select * from person
        """.trimIndent()
        info = session.getStatementInfo(sql)
        assertFalse(info.hasLimit)

        sql = """
            select * from person
            union
            (
                select * from person
                limit 1
            )
        """.trimIndent()
        info = session.getStatementInfo(sql)
        assertFalse(info.hasLimit)
    }

    @Test
    fun getSQLsOfScript() {
        val session = Session("getSQLsOfScript_test")

        val sqls = session.getSQLsOfScript(
            """
            create table person(
                id int primary key not null,
                name varchar(255) not null
            );
            
            create table info(id int not null);
            
            drop table info;
            """.trimIndent()
        )

        assertEquals("CREATE TABLE `person` (`id` INT PRIMARY KEY NOT NULL,`name` VARCHAR(255) NOT NULL)", sqls[0])
        assertEquals("CREATE TABLE `info` (`id` INT NOT NULL)", sqls[1])
        assertEquals("DROP TABLE `info`", sqls[2])

        session.close()
    }
}
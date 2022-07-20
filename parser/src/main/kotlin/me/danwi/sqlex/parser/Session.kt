package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.ffi.ffiCall
import me.danwi.sqlex.parser.ffi.ffiInvoke

class Field {
    lateinit var name: String
    lateinit var dbType: String
    var length: Long = -1
    var unsigned: Boolean = false
    var binary: Boolean = false
    var decimal: Long = -1
    lateinit var elements: Array<String>

    //下面是列包含的属性
    //是否为主键
    var isPrimaryKey = false

    //是否是联合主键的一部分
    var isPartOfMultipleKey = false

    //是否为自增
    var isAutoIncrement = false

    //是否唯一
    var isUnique = false

    //是否不能为空
    var notNull = false

    //是否含有默认值
    var hasDefaultValue = false
}

class TableInfo(
    val name: String,
    val primaryKey: Array<String>?,
    val uniques: Array<Array<String>>,
    val columns: Array<Field>
)

class PlanInfo(
    val fields: Array<Field>,
    val maxOneRow: Boolean
)

enum class StatementType { Select, Insert, Update, Delete, Other }

class InExprPosition(val not: Boolean, val marker: Int, val start: Int, val end: Int)

class IsNullExprPosition(val not: Boolean, val marker: Int, val start: Int, val end: Int)

class StatementInfo(
    val type: StatementType,
    val inExprPositions: Array<InExprPosition>,
    val isNullExprPositions: Array<IsNullExprPosition>,
    val hasLimit: Boolean,
    val limitRows: ULong,
)

class Session(database: String) {
    //创建Session
    private val sessionID: Long = ffiInvoke("DatabaseAPI", "CreateSession", database)

    fun execute(sql: String) {
        ffiCall("DatabaseAPI", "Execute", sessionID, sql)
    }

    fun executeScript(script: String) {
        ffiCall("DatabaseAPI", "ExecuteScript", sessionID, script)
    }

    val allTables: Array<String>
        get() = ffiInvoke("DatabaseAPI", "GetAllTable", sessionID)

    val DDL: String
        get() = ffiInvoke("DatabaseAPI", "GetDDL", sessionID)

    fun getTableDDL(table: String): String {
        return ffiInvoke("DatabaseAPI", "GetTableDDL", sessionID, table)
    }

    fun getTableInfo(table: String): TableInfo {
        return ffiInvoke("DatabaseAPI", "GetTableInfo", sessionID, table)
    }

    fun getPlanInfo(sql: String): PlanInfo {
        return ffiInvoke("DatabaseAPI", "GetPlanInfo", sessionID, sql)
    }

    fun getStatementInfo(sql: String): StatementInfo {
        return ffiInvoke("DatabaseAPI", "GetStatementInfo", sql)
    }

    fun getSQLsOfScript(script: String): Array<String> {
        return ffiInvoke("DatabaseAPI", "GetSQLsOfScript", script)
    }

    fun close() {
        ffiCall("DatabaseAPI", "CloseSession", sessionID)
    }
}
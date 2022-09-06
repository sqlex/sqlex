package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.ffi.ffiCall
import me.danwi.sqlex.parser.ffi.ffiInvoke

data class Field(
    val name: String,
    val dbType: String,
    val length: Long,
    val unsigned: Boolean,
    val binary: Boolean,
    val decimal: Long,
    val elements: Array<String>?,

    //下面是列包含的属性
    //是否为主键
    val isPrimaryKey: Boolean,
    //是否是联合主键的一部分
    val isMultipleKey: Boolean,
    //是否为自增
    val isAutoIncrement: Boolean,
    //是否唯一
    val isUnique: Boolean,
    //是否不能为空
    val notNull: Boolean,
    //是否含有默认值
    val hasDefaultValue: Boolean
)

data class TableInfo(
    val name: String,
    val primaryKey: Array<String>?,
    val uniques: Array<Array<String>>,
    val columns: Array<Field>
)

data class PlanInfo(
    val fields: Array<Field>,
    val maxOneRow: Boolean,
    val insertTable: String?
)

enum class StatementType { Select, Insert, Update, Delete, Other }

data class InExprPosition(val not: Boolean, val marker: Int, val start: Int, val end: Int)

data class IsNullExprPosition(val not: Boolean, val marker: Int, val start: Int, val end: Int)

data class StatementInfo(
    val type: StatementType,
    val inExprPositions: Array<InExprPosition>,
    val isNullExprPositions: Array<IsNullExprPosition>,
    val hasLimit: Boolean,
    val limitRows: Long,
)

class Session(val database: String) {
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

    fun dropDatabaseAndClose() {
        try {
            this.execute("drop database $database")
        } catch (_: Exception) {
        } finally {
            ffiCall("DatabaseAPI", "CloseSession", sessionID)
        }
    }
}
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
}

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
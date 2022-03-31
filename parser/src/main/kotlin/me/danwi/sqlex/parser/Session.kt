package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.ffi.ffiCall
import me.danwi.sqlex.parser.ffi.ffiInvoke

class Field {
    lateinit var name: String
    lateinit var dbType: String
    var length: Long = -1
    var decimal: Long = -1
    lateinit var elements: Array<String>
}

class InExprPosition(val not: Boolean, val marker: Int, val start: Int, val end: Int)

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

    fun getFields(sql: String): Array<Field> {
        return ffiInvoke("DatabaseAPI", "GetFields", sessionID, sql)
    }

    fun getInExprPositions(sql: String): Array<InExprPosition> {
        return ffiInvoke("DatabaseAPI", "GetInExprPositions", sql)
    }

    fun close() {
        ffiCall("DatabaseAPI", "CloseSession", sessionID)
    }
}
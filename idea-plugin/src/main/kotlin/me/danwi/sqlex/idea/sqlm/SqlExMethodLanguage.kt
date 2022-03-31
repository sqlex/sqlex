package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.Language

class SqlExMethodLanguage : Language("SqlExMethod") {
    companion object {
        @JvmStatic
        val INSTANCE = SqlExMethodLanguage()
    }
}
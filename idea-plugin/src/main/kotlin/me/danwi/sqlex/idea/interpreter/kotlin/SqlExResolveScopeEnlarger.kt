package me.danwi.sqlex.idea.interpreter.kotlin

import com.intellij.openapi.module.Module
import com.intellij.psi.search.SearchScope
import me.danwi.sqlex.idea.interpreter.SqlExCustomSearchScope
import org.jetbrains.kotlin.idea.base.projectStructure.KotlinResolveScopeEnlarger

class SqlExResolveScopeEnlarger : KotlinResolveScopeEnlarger {
    override fun getAdditionalResolveScope(module: Module, isTestScope: Boolean): SearchScope? {
        return SqlExCustomSearchScope(module.project)
    }
}
package me.danwi.sqlex.idea.interpreter

import com.intellij.openapi.module.Module
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.ResolveScopeEnlarger
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.search.SearchScope
import me.danwi.sqlex.idea.repositroy.SqlExGeneratedCacheKey

class SqlExMethodResolveScopeEnlarger : ResolveScopeEnlarger() {
    override fun getAdditionalResolveScope(file: VirtualFile, project: Project): SearchScope? {
        return SqlExCustomSearchScope()
    }
}

class SqlExCustomSearchScope : GlobalSearchScope() {
    override fun contains(file: VirtualFile): Boolean {
        return file.getUserData(SqlExGeneratedCacheKey) == true
    }

    override fun isSearchInModuleContent(module: Module): Boolean {
        return false
    }

    override fun isSearchInLibraries(): Boolean {
        return false
    }
}
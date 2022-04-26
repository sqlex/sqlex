package me.danwi.sqlex.idea.sqlm.generator

import com.intellij.openapi.module.Module
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.ResolveScopeEnlarger
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.search.SearchScope
import me.danwi.sqlex.idea.repositroy.SqlExMethodGeneratedCacheKey

class SqlExMethodResolveScopeEnlarger : ResolveScopeEnlarger() {
    override fun getAdditionalResolveScope(file: VirtualFile, project: Project): SearchScope? {
        return SqlExMethodCustomSearchScope()
    }
}

class SqlExMethodCustomSearchScope : GlobalSearchScope() {
    override fun contains(file: VirtualFile): Boolean {
        return file.getUserData(SqlExMethodGeneratedCacheKey) == true
    }

    override fun isSearchInModuleContent(module: Module): Boolean {
        return false
    }

    override fun isSearchInLibraries(): Boolean {
        return false
    }
}
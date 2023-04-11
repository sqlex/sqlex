package me.danwi.sqlex.idea.interpreter

import com.intellij.openapi.module.Module
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.ResolveScopeEnlarger
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.search.SearchScope
import me.danwi.sqlex.idea.repositroy.SqlExGeneratedCacheKey

class SqlExResolveScopeEnlarger : ResolveScopeEnlarger() {
    override fun getAdditionalResolveScope(file: VirtualFile, project: Project): SearchScope? {
        return SqlExCustomSearchScope(project)
    }
}

class SqlExCustomSearchScope(project: Project) : GlobalSearchScope(project) {
    override fun contains(file: VirtualFile): Boolean {
        return file.getUserData(SqlExGeneratedCacheKey) == true
                || file.path.contains("me/danwi/sqlex/core")
    }

    override fun isSearchInModuleContent(module: Module): Boolean {
        return false
    }

    override fun isSearchInLibraries(): Boolean {
        return false
    }
}
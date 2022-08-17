package me.danwi.sqlex.idea.sqlm

import com.intellij.navigation.ChooseByNameContributor
import com.intellij.navigation.NavigationItem
import com.intellij.openapi.project.Project
import com.intellij.psi.search.FileTypeIndex
import com.intellij.psi.search.GlobalSearchScope
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.psiFile

class SqlExMethodSymbolContributor : ChooseByNameContributor {
    //获取项目中的所有SqlEx Method
    private fun getAllMethodSubtree(project: Project): List<MethodSubtree> {
        return FileTypeIndex
            .getFiles(SqlExMethodFileType.INSTANCE, GlobalSearchScope.allScope(project))
            .mapNotNull { it.psiFile }
            .flatMap { it.childrenOf() }
    }

    override fun getNames(project: Project, includeNonProjectItems: Boolean): Array<String> {
        return getAllMethodSubtree(project).mapNotNull { it.name }.toTypedArray()
    }

    override fun getItemsByName(
        name: String?,
        pattern: String?,
        project: Project?,
        includeNonProjectItems: Boolean
    ): Array<NavigationItem> {
        if (project == null)
            return arrayOf()
        return getAllMethodSubtree(project).toTypedArray()
    }
}
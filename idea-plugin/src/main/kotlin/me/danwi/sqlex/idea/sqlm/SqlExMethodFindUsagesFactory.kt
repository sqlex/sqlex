package me.danwi.sqlex.idea.sqlm

import com.intellij.find.findUsages.FindUsagesHandler
import com.intellij.find.findUsages.FindUsagesHandlerFactory
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiReference
import com.intellij.psi.search.SearchScope
import com.intellij.psi.search.searches.ReferencesSearch
import me.danwi.sqlex.idea.service.SqlExMethodPsiClassCacheKey
import me.danwi.sqlex.idea.sqlm.psi.MethodNameSubtree
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.parentOf

class SqlExMethodFindUsagesFactory : FindUsagesHandlerFactory() {
    override fun canFindUsages(element: PsiElement): Boolean {
        return element is MethodNameSubtree || element is SqlExMethodFile
    }

    override fun createFindUsagesHandler(element: PsiElement, forHighlightUsages: Boolean): FindUsagesHandler? {
        if (element is MethodNameSubtree)
            return SqlExMethodMethodFindUsageHandler(element)
        else if (element is SqlExMethodFile)
            return SqlExMethodMFileFindUsageHandler(element)
        return null
    }
}

class SqlExMethodMethodFindUsageHandler(private val element: PsiElement) : FindUsagesHandler(element) {
    override fun getPrimaryElements(): Array<PsiElement> {
        return arrayOf(element.parentOf<MethodSubtree>() ?: return PsiElement.EMPTY_ARRAY)
    }

    override fun getSecondaryElements(): Array<PsiElement> {
        return arrayOf(element.parentOf<MethodSubtree>()?.javaMethod ?: return PsiElement.EMPTY_ARRAY)
    }

    override fun findReferencesToHighlight(
        target: PsiElement,
        searchScope: SearchScope
    ): MutableCollection<PsiReference> {
        val method = target.parentOf<MethodSubtree>() ?: return mutableListOf()
        val javaMethod = method.javaMethod ?: return mutableListOf()
        return ReferencesSearch.search(javaMethod).findAll()
    }
}

class SqlExMethodMFileFindUsageHandler(private val element: PsiElement) : FindUsagesHandler(element) {
    override fun getSecondaryElements(): Array<PsiElement> {
        return arrayOf(
            element.containingFile.virtualFile.getUserData(SqlExMethodPsiClassCacheKey) ?: return PsiElement.EMPTY_ARRAY
        )
    }

    override fun findReferencesToHighlight(
        target: PsiElement,
        searchScope: SearchScope
    ): MutableCollection<PsiReference> {
        val psiClass =
            element.containingFile.virtualFile.getUserData(SqlExMethodPsiClassCacheKey) ?: return mutableListOf()
        return ReferencesSearch.search(psiClass).findAll()
    }
}
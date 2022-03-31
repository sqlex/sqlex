package me.danwi.sqlex.idea.sqlm.generator

import com.intellij.openapi.project.Project
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiField
import com.intellij.psi.PsiMethod
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.search.PsiShortNamesCache
import com.intellij.util.Processor
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryServices

class SqlExMethodGeneratedShortNamesCache(private val project: Project) : PsiShortNamesCache() {
    override fun getClassesByName(name: String, scope: GlobalSearchScope): Array<PsiClass> {
        return project.sqlexRepositoryServices
            .filter { scope.contains(it.sourceRoot) }
            .mapNotNull { it.findClassByName(name) }
            .toTypedArray()
    }

    override fun getAllClassNames(): Array<String> {
        return project.sqlexRepositoryServices
            .flatMap { it.allJavaClass.map { c -> c.name } }
            .filterNotNull()
            .distinct()
            .toTypedArray()
    }

    override fun processMethodsWithName(
        name: String,
        scope: GlobalSearchScope,
        processor: Processor<in PsiMethod>
    ): Boolean {
        return false
    }

    override fun getMethodsByName(name: String, scope: GlobalSearchScope): Array<PsiMethod> {
        return arrayOf()
    }

    override fun getMethodsByNameIfNotMoreThan(
        name: String,
        scope: GlobalSearchScope,
        maxCount: Int
    ): Array<PsiMethod> {
        return arrayOf()
    }

    override fun getAllMethodNames(): Array<String> {
        return arrayOf()
    }

    override fun getAllFieldNames(): Array<String> {
        return arrayOf()
    }

    override fun getFieldsByName(name: String, scope: GlobalSearchScope): Array<PsiField> {
        return arrayOf()
    }

    override fun getFieldsByNameIfNotMoreThan(name: String, scope: GlobalSearchScope, maxCount: Int): Array<PsiField> {
        return arrayOf()
    }
}
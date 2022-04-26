package me.danwi.sqlex.idea.sqlm.generator

import com.intellij.openapi.project.Project
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElementFinder
import com.intellij.psi.PsiPackage
import com.intellij.psi.search.GlobalSearchScope
import me.danwi.sqlex.idea.repositroy.sqlexRepositoryServices

class SqlExMethodGeneratedElementFinder(private val project: Project) : PsiElementFinder() {
    override fun findClasses(qualifiedName: String, scope: GlobalSearchScope): Array<PsiClass> {
        return project.sqlexRepositoryServices
            .filter { scope.contains(it.sourceRoot) }
            .mapNotNull { it.repository?.findClass(qualifiedName) }
            .toTypedArray()
    }

    override fun findClass(qualifiedName: String, scope: GlobalSearchScope): PsiClass? {
        return findClasses(qualifiedName, scope).firstOrNull()
    }

    override fun getClasses(psiPackage: PsiPackage, scope: GlobalSearchScope): Array<PsiClass> {
        return project.sqlexRepositoryServices
            .filter { scope.contains(it.sourceRoot) }
            .mapNotNull { it.repository }
            .flatMap { it.findClasses(psiPackage.qualifiedName) }
            .toTypedArray()
    }
}
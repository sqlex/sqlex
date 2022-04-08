package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiElement
import com.intellij.psi.util.PsiTreeUtil

inline fun <reified T : PsiElement> PsiElement.childrenOf(): Collection<T> {
    return PsiTreeUtil.findChildrenOfType(this, T::class.java)
}

inline fun <reified T : PsiElement> PsiElement.parentOf(): T? {
    return PsiTreeUtil.getParentOfType(this, T::class.java)
}

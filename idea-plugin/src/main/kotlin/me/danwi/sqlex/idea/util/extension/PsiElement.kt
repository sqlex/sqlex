package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiElement

fun PsiElement.visit(visitor: (PsiElement) -> Unit) {
    visitor(this)
    children.forEach { it.visit(visitor) }
}
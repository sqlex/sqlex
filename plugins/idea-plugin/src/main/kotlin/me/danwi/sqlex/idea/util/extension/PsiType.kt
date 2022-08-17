package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiClass
import com.intellij.psi.PsiType
import com.intellij.psi.util.PsiTypesUtil

val PsiType.psiClass: PsiClass?
    inline get() = PsiTypesUtil.getPsiClass(this)

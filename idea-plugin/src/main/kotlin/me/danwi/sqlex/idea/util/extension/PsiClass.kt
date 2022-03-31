package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiClass

//获取权限定包名
val PsiClass.qualifiedPackageName: String?
    inline get() = this.qualifiedName?.removePrefix(".${this.name}")
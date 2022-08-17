package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiMethod
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree

//获取PsiMethod对应的SqlEx Method Subtree
val PsiMethod.methodSubtree: MethodSubtree?
    inline get() = this.containingClass
        ?.sqlexMethodFile?.psiFile
        ?.childrenOf<MethodSubtree>()
        ?.find { m -> m.methodName?.methodName == this.name }
package me.danwi.sqlex.idea.config

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.*
import me.danwi.sqlex.idea.util.extension.*
import org.jetbrains.yaml.psi.YAMLKeyValue
import org.jetbrains.yaml.psi.YAMLScalar

class SqlExConfigConvertersInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (!element.containingFile.virtualFile.isSqlExConfig)
                    return
                if (element !is YAMLScalar)
                    return
                if (element.parentOf<YAMLKeyValue>()?.name != "converters")
                    return

                //获取到注入的java class
                val injectedClass =
                    element
                        .injectedOf<PsiJavaFile>()
                        ?.childrenOf<PsiJavaCodeReferenceElement>()
                        ?.firstOrNull()
                        ?.resolve()
                if (injectedClass !is PsiClass) {
                    holder.registerProblem(
                        element,
                        "请填写正确的参数类型转换类",
                        ProblemHighlightType.ERROR
                    )
                    return
                }
                if (!injectedClass.isParameterConverter) {
                    holder.registerProblem(
                        element,
                        "${injectedClass.name} 不是一个合法的参数类型转换器",
                        ProblemHighlightType.ERROR
                    )
                    return
                }
            }
        }
    }
}
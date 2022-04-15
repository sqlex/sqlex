package me.danwi.sqlex.idea.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.common.ParameterTypes.ParameterConverterInterfaceQualifiedName
import me.danwi.sqlex.idea.util.extension.*

class SqlExJavaConverterImplementInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is PsiClass)
                    return

                //判断其是否为一个ParameterConverter的实现
                val converterType =
                    element.implementsListTypes.find { it.psiClass?.isParameterConverterInterface == true } ?: return

                //先判断其是否含有无参构造函数
                if (!element.hasNoArgsConstructor) {
                    //准备做无参构造函数提醒
                    val anchor = element.nameIdentifier ?: element
                    holder.registerProblem(
                        anchor,
                        "参数类型转换器必须有一个无参构造函数",
                        ProblemHighlightType.ERROR
                    )
                }

                //获取他的toType类型
                val toTypeClass = converterType.converterToType?.psiClass
                if (toTypeClass != null) {
                    //判断他是否是预支持类型
                    if (!toTypeClass.isPreSupported) {
                        //准备做非法类型提醒
                        //先找到对应的element
                        var anchor = element
                        val implementList = element.implementsList
                        if (implementList != null) {
                            val typeParameterElements =
                                implementList
                                    .referenceElements
                                    .find { it.qualifiedName == ParameterConverterInterfaceQualifiedName }
                                    ?.parameterList
                                    ?.typeParameterElements
                            if (typeParameterElements?.size == 2)
                                anchor = typeParameterElements[1]
                        }
                        holder.registerProblem(
                            anchor,
                            "目标类型${toTypeClass.name}不是预支持的数据类型,不能作为参数",
                            ProblemHighlightType.ERROR
                        )
                    }
                }
            }
        }
    }
}
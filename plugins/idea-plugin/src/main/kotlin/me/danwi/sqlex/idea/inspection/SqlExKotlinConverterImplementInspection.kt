package me.danwi.sqlex.idea.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.JavaPsiFacade
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.core.type.ParameterConverter
import me.danwi.sqlex.idea.util.extension.*
import org.jetbrains.kotlin.psi.KtClass

class SqlExKotlinConverterImplementInspection() : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is KtClass)
                    return

                //获取这个kt class的super type中等于转换器名称的元素
                //加这个判断
                //      一是为了加速判断,如果名字不相等,就不用进入到下面获取/判断PsiClass的耗时操作了
                //      二是为了获取到psi tree中的元素,方便注册问题的时候定位到元素上
                val converterInterfaceElement = element.superTypeListEntries
                    .map { it.typeAsUserType }
                    .find { it?.referencedName == ParameterConverter::class.java.simpleName }
                    ?: return

                //获取这个class的jvm全限定名
                val qualifiedName = element.fqName ?: return

                //获取到他的psiClass
                val psiClass = JavaPsiFacade.getInstance(element.project)
                    .findClass(qualifiedName.asString(), element.resolveScope)
                    ?: return

                //判断其是否为一个ParameterConverter的实现
                val converterType = psiClass.implementsListTypes
                    .find { it.psiClass?.isParameterConverterInterface == true } ?: return

                //判断其是否含有无参构造函数
                if (!psiClass.hasNoArgsConstructor) {
                    val anchor = element.identifyingElement ?: element
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
                        var anchor: PsiElement = converterInterfaceElement
                        val typeArgs = converterInterfaceElement.typeArgumentList?.arguments ?: listOf()
                        if (typeArgs.size == 2)
                            anchor = typeArgs[1]
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
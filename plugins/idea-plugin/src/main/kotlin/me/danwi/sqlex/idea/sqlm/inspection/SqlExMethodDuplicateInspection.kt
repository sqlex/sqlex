package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.psi.PsiFile
import com.intellij.psi.util.parents
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.idea.sqlm.psi.ParamListSubtree
import me.danwi.sqlex.idea.sqlm.psi.ParamNameSubtree
import me.danwi.sqlex.parser.util.pascalName

class SqlExMethodDuplicateInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitFile(file: PsiFile) {
                if (file !is SqlExMethodFile) return

                //获取到所有的方法定义
                val methods = file.root?.methods ?: listOf()
                val methodNames = methods.mapNotNull { it.methodName }

                //方法重名检测
                methodNames.filter { a -> methodNames.filter { b -> a.methodName == b.methodName }.size > 1 }
                    .forEach { holder.registerProblem(it, "重复的方法名") }

                //检测重名返回类型
                val returnTypeNames =
                    methods.map { it.returnType?.typeName ?: "${it.methodName?.methodName?.pascalName}Result" }
                val duplicatedTypeNames =
                    returnTypeNames.filter { a -> returnTypeNames.filter { b -> a == b }.size > 1 }
                methods.mapNotNull { it.returnType }.filter { r -> duplicatedTypeNames.any { r.typeName == it } }
                    .forEach { holder.registerProblem(it, "重复的返回值名称") }
            }

            override fun visitElement(element: PsiElement) {
                if (element !is ParamNameSubtree)
                    return
                //获取参数列表
                val paramList = element.parents(false).filterIsInstance<ParamListSubtree>().firstOrNull() ?: return
                //检查名称是否重复
                if (paramList.params.mapNotNull { it.paramName?.text }.filter { it == element.text }.size > 1)
                    holder.registerProblem(element, "存在重复的参数名")
            }
        }
    }
}
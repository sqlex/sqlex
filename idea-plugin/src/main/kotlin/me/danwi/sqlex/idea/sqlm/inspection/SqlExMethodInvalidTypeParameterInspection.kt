package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemHighlightType
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.idea.sqlm.psi.ParamTypeSubtree
import me.danwi.sqlex.idea.util.extension.configFile
import me.danwi.sqlex.idea.util.extension.converters
import me.danwi.sqlex.idea.util.extension.psiFile
import me.danwi.sqlex.parser.PreSupportedTypes
import org.jetbrains.yaml.psi.YAMLFile

class SqlExMethodInvalidTypeParameterInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                if (element !is ParamTypeSubtree) return
                //获取java class
                val javaClass = element.javaType
                //判断是否是一个合法的java class
                if (javaClass == null) {
                    holder.registerProblem(element, "参数类型无法确定,请确保其正确导入", ProblemHighlightType.ERROR)
                    return
                }
                //判断是否是预支持数据类型
                if (PreSupportedTypes.contains(javaClass.qualifiedName)) {
                    return
                }
                //获取SqlEx config文件
                val configFile = element.containingFile.virtualFile.configFile?.psiFile ?: return
                if (configFile !is YAMLFile)
                    return
                //获取参数转换器支持的参数类型
                val convertFromList = configFile.converters?.map { it.fromTypeQualifiedName } ?: listOf()
                if (!check(javaClass, convertFromList)) {
                    holder.registerProblem(element, "该参数没有对应参数转换器", ProblemHighlightType.ERROR)
                    return
                }
            }
        }
    }

    private fun check(clazz: PsiClass?, converters: List<String>): Boolean {
        if (clazz == null) return false
        if (converters.contains(clazz.qualifiedName)) return true
        return check(clazz.superClass, converters)
    }
}
package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.LocalQuickFix
import com.intellij.codeInspection.ProblemDescriptor
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import com.intellij.psi.impl.source.tree.LeafElement
import me.danwi.sqlex.idea.sqlm.psi.MethodNameSubtree

class SqlExMethodNamingSuggestionInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //处理方法名
                if (element !is MethodNameSubtree)
                    return

                if (element.methodName.startsWith("get"))
                    holder.registerProblem(
                        element,
                        "数据查询方法建议以find开头,例如 ${element.methodName} -> find${element.methodName.removePrefix("get")}",
                        object : LocalQuickFix {
                            override fun getFamilyName(): String {
                                return "将方法名修改为find开头"
                            }

                            override fun applyFix(project: Project, descriptor: ProblemDescriptor) {
                                val textElement = descriptor.psiElement.children.firstOrNull() ?: return
                                if (textElement !is LeafElement)
                                    return
                                textElement.replaceWithText("find${(textElement as LeafElement).text.removePrefix("get")}")
                            }
                        })

                if (element.methodName.startsWith("set"))
                    holder.registerProblem(
                        element,
                        "数据修改方法建议以update开口,例如 ${element.methodName} -> update${element.methodName.removePrefix("set")}",
                        object : LocalQuickFix {
                            override fun getFamilyName(): String {
                                return "将方法名修改为update开头"
                            }

                            override fun applyFix(project: Project, descriptor: ProblemDescriptor) {
                                val textElement = descriptor.psiElement.children.firstOrNull() ?: return
                                if (textElement !is LeafElement)
                                    return
                                textElement.replaceWithText("update${(textElement as LeafElement).text.removePrefix("set")}")
                            }
                        })
            }
        }
    }
}
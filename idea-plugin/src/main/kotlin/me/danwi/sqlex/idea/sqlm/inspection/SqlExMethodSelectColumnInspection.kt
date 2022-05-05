package me.danwi.sqlex.idea.sqlm.inspection

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.openapi.util.Key
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiElementVisitor
import me.danwi.sqlex.common.ColumnNameRegex
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.StatementType

private val sqlCacheKey = Key<String>("me.danwi.sqlex.idea.sqlm.inspection.SqlCacheKey")
private val errCacheKey = Key<String>("me.danwi.sqlex.idea.sqlm.inspection.ErrCacheKey")

class SqlExMethodSelectColumnInspection : LocalInspectionTool() {
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        return object : PsiElementVisitor() {
            override fun visitElement(element: PsiElement) {
                //找到方法
                if (element !is MethodSubtree) return
                //获取sql元素
                val sqlSubtree = element.sql ?: return
                //获取sql文本
                val sql = sqlSubtree.text ?: return
                //处理sql
                val cacheValue = sql.split("\n").joinToString(" ") { it.trimIndent().trim() }
                //获取缓存sql
                val cachedSql = element.getUserData(sqlCacheKey)
                //获取缓存错误信息
                var message = element.getUserData(errCacheKey)
                //判断缓存是否一致
                if (cachedSql == cacheValue) {
                    if (message != null && message.isNotEmpty()) {
                        holder.registerProblem(sqlSubtree, message)
                    }
                } else {
                    try {
                        //获取session
                        val session =
                            element.containingFile.virtualFile.sqlexRepositoryService?.repository?.session ?: return
                        //判断是否是select
                        if (session.getStatementInfo(sql).type != StatementType.Select) return
                        //获取列名
                        val fields = session.getFields(sql)
                        //判断列名是否重复
                        val duplicateFieldNames = fields.groupingBy { it.name }.eachCount().filter { it.value > 1 }
                        //判断列名是否非法
                        val regex = ColumnNameRegex.ColumnNameRegex.toRegex()
                        val invalidFieldNames = fields.map { it.name }.filter { !regex.containsMatchIn(it) }

                        if (duplicateFieldNames.isNotEmpty()) {
                            message = "存在重复的列名 ${duplicateFieldNames.map { "'${it.key}'" }.joinToString(", ")}"
                            holder.registerProblem(sqlSubtree, message)
                        } else if (invalidFieldNames.isNotEmpty()) {
                            message = "存在非法的列名 ${invalidFieldNames.joinToString(", ")}"
                            holder.registerProblem(sqlSubtree, message)
                        } else {
                            message = null
                        }
                    } catch (e: Exception) {
                        message = e.message ?: "解析错误"
                        holder.registerProblem(sqlSubtree, message)
                    }
                    //存入缓存
                    element.putUserData(sqlCacheKey, cacheValue)
                    element.putUserData(errCacheKey, message)
                }
            }
        }
    }
}
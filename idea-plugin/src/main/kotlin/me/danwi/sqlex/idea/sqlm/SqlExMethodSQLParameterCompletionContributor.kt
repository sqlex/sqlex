package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.completion.*
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.icons.AllIcons
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.util.parents
import com.intellij.sql.psi.SqlParameter
import com.intellij.util.ProcessingContext
import me.danwi.sqlex.idea.util.extension.containingSqlExMethod

class SqlExMethodSQLParameterCompletionContributor : CompletionContributor() {
    init {
        val contributor = object : CompletionProvider<CompletionParameters>() {
            override fun addCompletions(
                parameters: CompletionParameters,
                context: ProcessingContext,
                resultSet: CompletionResultSet
            ) {
                //获取sql参数psi element
                val sqlParameter =
                    parameters.position.parents(false).find { it is SqlParameter } as SqlParameter? ?: return
                //获取sql参数所注入的method
                val method = sqlParameter.containingSqlExMethod ?: return
                //拿到method的参数列表
                val params = method.paramList?.params ?: return
                //lookup elements
                val elements = params
                    .mapNotNull {
                        val paramName = it.paramName?.text?.trim() ?: return@mapNotNull null
                        var element = LookupElementBuilder.create(paramName).withIcon(AllIcons.FileTypes.Java)
                        val paramType = it.paramType?.text?.trim()
                        if (paramType != null) {
                            element = if (it.isCollectionParam)
                                element.withTypeText("$paramType[]")
                            else
                                element.withTypeText(paramType)
                        }
                        element
                    }
                //添加了结果中
                resultSet.addAllElements(elements)
            }
        }
        //注入自动完成
        extend(CompletionType.BASIC, PlatformPatterns.psiElement(), contributor)
        extend(CompletionType.SMART, PlatformPatterns.psiElement(), contributor)
    }
}
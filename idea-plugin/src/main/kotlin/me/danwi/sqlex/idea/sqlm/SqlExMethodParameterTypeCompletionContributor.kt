package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.completion.*
import com.intellij.codeInsight.lookup.LookupElement
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiClass
import com.intellij.util.ProcessingContext
import me.danwi.sqlex.idea.sqlm.psi.ParamTypeSubtree
import me.danwi.sqlex.idea.util.extension.parentOf

class SqlExMethodParameterTypeCompletionContributor : CompletionContributor() {
    init {
        val contributor = object : CompletionProvider<CompletionParameters>() {
            override fun addCompletions(
                parameters: CompletionParameters,
                context: ProcessingContext,
                resultSet: CompletionResultSet
            ) {
                if (parameters.position.parentOf<ParamTypeSubtree>() == null)
                    return

                //搜索所有的java类
                AllClassesGetter.processJavaClasses(
                    parameters,
                    resultSet.prefixMatcher,
                    true
                ) {
                    val lookupElement = JavaLookupElementBuilder
                        .forClass(it, it.name, true)
                        .withInsertHandler(ParameterTypeInsertHandler(it))
                    resultSet.addElement(lookupElement)
                }
            }
        }
        //注入自动完成
        extend(CompletionType.BASIC, PlatformPatterns.psiElement(), contributor)
        extend(CompletionType.SMART, PlatformPatterns.psiElement(), contributor)
    }
}

private class ParameterTypeInsertHandler(private val psiClass: PsiClass) : InsertHandler<LookupElement> {
    override fun handleInsert(context: InsertionContext, element: LookupElement) {
        val sqlexMethodFile = context.file
        if (sqlexMethodFile !is SqlExMethodFile)
            return
        val qualifiedName = psiClass.qualifiedName ?: return
        sqlexMethodFile.root?.addImport(qualifiedName)
    }
}
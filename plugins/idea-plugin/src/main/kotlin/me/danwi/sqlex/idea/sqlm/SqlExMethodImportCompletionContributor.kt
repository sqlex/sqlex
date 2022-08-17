package me.danwi.sqlex.idea.sqlm

import com.intellij.codeInsight.completion.*
import com.intellij.patterns.PlatformPatterns
import com.intellij.util.ProcessingContext
import me.danwi.sqlex.idea.sqlm.psi.ClassNameSubtree
import me.danwi.sqlex.idea.util.extension.parentOf

class SqlExMethodImportCompletionContributor : CompletionContributor() {
    init {
        val contributor = object : CompletionProvider<CompletionParameters>() {
            override fun addCompletions(
                parameters: CompletionParameters,
                context: ProcessingContext,
                resultSet: CompletionResultSet
            ) {
                if (parameters.position.parentOf<ClassNameSubtree>() == null)
                    return

                //搜索所有的java类
                AllClassesGetter.processJavaClasses(
                    parameters,
                    resultSet.prefixMatcher,
                    true
                ) {
                    val lookupElement = JavaLookupElementBuilder.forClass(it, it.qualifiedName)
                    resultSet.addElement(lookupElement)
                }
            }
        }
        //注入自动完成
        extend(CompletionType.BASIC, PlatformPatterns.psiElement(), contributor)
        extend(CompletionType.SMART, PlatformPatterns.psiElement(), contributor)
    }
}
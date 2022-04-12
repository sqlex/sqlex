package me.danwi.sqlex.idea.config

import com.intellij.lang.injection.MultiHostInjector
import com.intellij.lang.injection.MultiHostRegistrar
import com.intellij.lang.java.JavaLanguage
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import me.danwi.sqlex.idea.util.extension.isSqlExConfig
import me.danwi.sqlex.idea.util.extension.parentOf
import org.jetbrains.yaml.psi.YAMLKeyValue
import org.jetbrains.yaml.psi.YAMLScalar

class SqlExConfigJavaInjector : MultiHostInjector {
    override fun elementsToInjectIn(): MutableList<out Class<out PsiElement>> {
        return mutableListOf(YAMLScalar::class.java)
    }

    override fun getLanguagesToInject(registrar: MultiHostRegistrar, context: PsiElement) {
        if (!context.containingFile.virtualFile.isSqlExConfig)
            return
        if (context !is YAMLScalar)
            return
        if (context.parentOf<YAMLKeyValue>()?.name != "converters")
            return
        registrar
            .startInjecting(JavaLanguage.INSTANCE)
            .addPlace(
                "import ", ";",
                context,
                TextRange.from(0, context.textLength)
            )
            .doneInjecting()
    }
}
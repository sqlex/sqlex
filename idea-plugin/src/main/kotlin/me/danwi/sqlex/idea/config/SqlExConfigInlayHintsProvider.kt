@file:Suppress("UnstableApiUsage")

package me.danwi.sqlex.idea.config

import com.intellij.codeInsight.hints.*
import com.intellij.database.dialects.base.endOffset
import com.intellij.openapi.editor.Editor
import com.intellij.psi.*
import me.danwi.sqlex.idea.util.extension.*
import org.jetbrains.yaml.psi.YAMLKeyValue
import org.jetbrains.yaml.psi.YAMLScalar
import javax.swing.JPanel

private val settingsKey = SettingsKey<NoSettings>("me.danwi.sqlex.SqlExConfigInlayProviderSettingsKey")

class SqlExConfigInlayHintsProvider : InlayHintsProvider<NoSettings> {
    override fun getCollectorFor(
        file: PsiFile,
        editor: Editor,
        settings: NoSettings,
        sink: InlayHintsSink
    ): InlayHintsCollector? {
        if (!file.virtualFile.isSqlExConfig)
            return null
        return Collector(editor)
    }


    override val key: SettingsKey<NoSettings> = settingsKey
    override val name: String = "SqlEx Config Inlay Hints"
    override val previewText: String? = null
    override fun createSettings() = NoSettings()

    override fun createConfigurable(settings: NoSettings) = object : ImmediateConfigurable {
        override fun createComponent(listener: ChangeListener) = JPanel()
    }
}

class Collector(editor: Editor) : FactoryInlayHintsCollector(editor) {
    override fun collect(element: PsiElement, editor: Editor, sink: InlayHintsSink): Boolean {
        if (!element.containingFile.virtualFile.isSqlExConfig)
            return false
        if (element !is YAMLScalar)
            return true
        if (element.parentOf<YAMLKeyValue>()?.name != "converters")
            return true

        //获取到对应到java注入
        val injectedClass =
            element.injectedOf<PsiJavaFile>()?.childrenOf<PsiJavaCodeReferenceElement>()?.firstOrNull()?.resolve()
                ?: return true
        if (injectedClass !is PsiClass)
            return true
        //获取到这个class实现的ParameterConverter接口
        val converter =
            injectedClass.implementsListTypes.firstOrNull { it.psiClass?.isParameterConverterInterface == true }
                ?: return true
        val fromTypeClass = converter.converterFromType?.psiClass
        val toTypeClass = converter.converterToType?.psiClass

        //构建inlay信息
        val typeInlays = listOf(fromTypeClass, toTypeClass)
            .map {
                //构建inlay信息
                var typeInlay = this.factory.text(it?.name ?: "?")
                typeInlay = this.factory.roundWithBackground(typeInlay)
                if (it != null) {
                    typeInlay = this.factory.withTooltip(it.qualifiedName ?: "", typeInlay)
                    typeInlay = this.factory.psiSingleReference(typeInlay) { it }
                }
                typeInlay
            }

        val inlay =
            this.factory.join(typeInlays) { this.factory.roundWithBackground(this.factory.text("to")) }

        //添加inlay信息
        sink.addInlineElement(
            element.endOffset,
            true,
            inlay,
            false
        )

        return true
    }
}
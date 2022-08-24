package me.danwi.sqlex.idea.sqls

import com.intellij.codeInsight.hints.*
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.refactoring.suggested.startOffset
import com.intellij.sql.psi.SqlDdlStatement
import com.intellij.sql.psi.SqlDmlStatement
import com.intellij.sql.psi.SqlStatement
import icons.SqlExIcons
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.isSqlExSchema
import me.danwi.sqlex.parser.util.schemaFileVersion
import javax.swing.JPanel

@Suppress("UnstableApiUsage")
class SqlExSchemaVersionInlayHintsProvider : InlayHintsProvider<NoSettings> {
    override val key: SettingsKey<NoSettings> =
        SettingsKey("me.danwi.sqlex.SqlExSqlExSchemaVersionInlaySettingsKey")
    override val name: String = "SqlEx Schema version inlay hints"
    override val previewText: String? = null
    override fun createSettings() = NoSettings()
    override fun createConfigurable(settings: NoSettings) = object : ImmediateConfigurable {
        override fun createComponent(listener: ChangeListener) = JPanel()
    }

    override fun getCollectorFor(
        file: PsiFile,
        editor: Editor,
        settings: NoSettings,
        sink: InlayHintsSink
    ): InlayHintsCollector? {
        if (file.virtualFile?.isSqlExSchema != true)
            return null

        //Schema版本
        val version = file.virtualFile?.path?.schemaFileVersion ?: return null
        //获取sqls中所有的语句
        val statements = file.childrenOf<SqlStatement>()

        //返回inlay收集器
        return object : FactoryInlayHintsCollector(editor) {
            override fun collect(element: PsiElement, editor: Editor, sink: InlayHintsSink): Boolean {
                if (element !is SqlStatement)
                    return true
                //找到语句在整个文件中的序号
                val index = statements.indexOf(element)
                if (index == -1)
                    return true
                //构建inlay信息
                val inlay = this.factory.seq(
                    this.factory.icon(SqlExIcons.IconX12),
                    when (element) {
                        is SqlDdlStatement -> this.factory.smallText("(DDL)")
                        is SqlDmlStatement -> this.factory.smallText("(DML)")
                        else -> this.factory.smallText("(其他)")
                    },
                    this.factory.textSpacePlaceholder(1, true),
                    this.factory.smallText("子版本 $version.$index"),
                )
                sink.addBlockElement(
                    element.startOffset,
                    relatesToPrecedingText = true,
                    showAbove = true,
                    priority = 0,
                    presentation = inlay
                )
                return true
            }
        }
    }
}
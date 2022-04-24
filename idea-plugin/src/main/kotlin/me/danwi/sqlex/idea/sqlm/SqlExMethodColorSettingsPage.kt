package me.danwi.sqlex.idea.sqlm

import com.intellij.openapi.editor.colors.EditorColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighter
import com.intellij.openapi.options.colors.AttributesDescriptor
import com.intellij.openapi.options.colors.ColorDescriptor
import com.intellij.openapi.options.colors.ColorSettingsPage
import icons.SqlExIcons
import javax.swing.Icon

class SqlExMethodColorSettingsPage : ColorSettingsPage {
    private val INJECT = EditorColors.createInjectedLanguageFragmentKey(SqlExMethodLanguage.INSTANCE)

    override fun getDisplayName(): String {
        return "SqlEx Method"
    }

    override fun getIcon(): Icon {
        return SqlExIcons.MethodFile
    }

    override fun getHighlighter(): SyntaxHighlighter {
        return SqlExMethodSyntaxHighlighter()
    }

    override fun getAttributeDescriptors(): Array<AttributesDescriptor> {
        return arrayOf(AttributesDescriptor("injected_fragment", INJECT))
    }

    override fun getColorDescriptors(): Array<ColorDescriptor> {
        return ColorDescriptor.EMPTY_ARRAY
    }

    override fun getDemoText(): String {
        return """
            import java.math.BigDecimal
            import java.util.Date

            Person findAll(age:Int, names:String*){
                select name, ip
                from person
                where age > :age
                  and name in (:name)
            }
        """.trimIndent()
    }

    override fun getAdditionalHighlightingTagToDescriptorMap(): MutableMap<String, TextAttributesKey>? {
        return mutableMapOf()
    }
}
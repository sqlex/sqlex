package me.danwi.sqlex.idea.sqlm

import com.intellij.icons.AllIcons
import com.intellij.openapi.fileTypes.LanguageFileType
import me.danwi.sqlex.parser.util.SqlExMethodExtensionName
import javax.swing.Icon

class SqlExMethodFileType : LanguageFileType(SqlExMethodLanguage.INSTANCE) {
    companion object {
        @JvmStatic
        val INSTANCE = SqlExMethodFileType()
    }

    override fun getName(): String {
        return "SqlEx Method"
    }

    override fun getDescription(): String {
        return "SqlEx method file"
    }

    override fun getDefaultExtension(): String {
        return SqlExMethodExtensionName
    }

    override fun getIcon(): Icon {
        return AllIcons.Providers.Mysql
    }
}
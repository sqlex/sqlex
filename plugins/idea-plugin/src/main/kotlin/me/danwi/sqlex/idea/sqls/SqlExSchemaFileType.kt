package me.danwi.sqlex.idea.sqls

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.sql.psi.SqlLanguage
import icons.SqlExIcons
import me.danwi.sqlex.parser.util.SqlExSchemaExtensionName
import javax.swing.Icon

class SqlExSchemaFileType : LanguageFileType(SqlLanguage.INSTANCE) {
    companion object {
        @JvmStatic
        val INSTANCE = SqlExSchemaFileType()
    }

    override fun getName(): String {
        return "SqlEx Schema"
    }

    override fun getDescription(): String {
        return "SqlEx Schema migrate file"
    }

    override fun getDefaultExtension(): String {
        return SqlExSchemaExtensionName
    }

    override fun getIcon(): Icon {
        return SqlExIcons.SchemaFile
    }
}
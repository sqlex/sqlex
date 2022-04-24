package me.danwi.sqlex.idea.config

import com.intellij.openapi.fileTypes.LanguageFileType
import icons.SqlExIcons
import me.danwi.sqlex.parser.util.SqlExConfigFileExtensionName
import org.jetbrains.yaml.YAMLLanguage
import javax.swing.Icon

class SqlExConfigFileType : LanguageFileType(YAMLLanguage.INSTANCE) {
    companion object {
        @JvmStatic
        val INSTANCE = SqlExConfigFileType()
    }

    override fun getName(): String {
        return "SqlEx Config"
    }

    override fun getDescription(): String {
        return "SqlEx Config file"
    }

    override fun getDefaultExtension(): String {
        return SqlExConfigFileExtensionName
    }

    override fun getIcon(): Icon {
        return SqlExIcons.ConfigFile
    }
}
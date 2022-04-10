package me.danwi.sqlex.idea.sqlm

import com.intellij.extapi.psi.PsiFileBase
import com.intellij.openapi.fileTypes.FileType
import com.intellij.psi.FileViewProvider
import me.danwi.sqlex.idea.sqlm.psi.RootSubtree
import me.danwi.sqlex.idea.util.SqlExMethodFileIcon
import javax.swing.Icon

class SqlExMethodFile(viewProvider: FileViewProvider) : PsiFileBase(viewProvider, SqlExMethodLanguage.INSTANCE) {
    override fun getFileType(): FileType {
        return SqlExMethodFileType.INSTANCE
    }

    override fun getIcon(flags: Int): Icon {
        return SqlExMethodFileIcon
    }

    override fun toString(): String {
        return "SqlEx Method File"
    }

    val root: RootSubtree?
        get() = this.children.filterIsInstance<RootSubtree>().firstOrNull()
}
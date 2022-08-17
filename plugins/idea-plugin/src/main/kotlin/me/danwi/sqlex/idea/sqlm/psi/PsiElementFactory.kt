package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.openapi.project.Project
import com.intellij.psi.PsiFileFactory
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.idea.sqlm.SqlExMethodLanguage

object PsiElementFactory {
    private fun createFile(content: String, project: Project): SqlExMethodFile? {
        return try {
            val psiFile = PsiFileFactory.getInstance(project)
                .createFileFromText("dummy.sqlm", SqlExMethodLanguage.INSTANCE, content)
            psiFile as SqlExMethodFile
        } catch (_: Exception) {
            null
        }
    }

    fun createImport(className: String, project: Project): ImportSubtree? {
        val file = createFile(
            """
            import $className
        """.trimIndent(), project
        ) ?: return null
        return file.root?.imports?.firstOrNull()
    }
}
package me.danwi.sqlex.idea

import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.StartupActivity
import me.danwi.sqlex.idea.repositroy.showMaybeSqlExImportNotification
import me.danwi.sqlex.idea.repositroy.syncRepositoryService
import me.danwi.sqlex.idea.util.extension.modules
import me.danwi.sqlex.idea.util.extension.unmarkAllGeneratedSourceRoot

class SqlExStartupActivity : StartupActivity {
    override fun runActivity(project: Project) {
        project.modules.forEach { it.unmarkAllGeneratedSourceRoot() }
        project.syncRepositoryService()
        project.showMaybeSqlExImportNotification()
    }
}
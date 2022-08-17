package me.danwi.sqlex.idea.repositroy.listener

import com.intellij.openapi.roots.ModuleRootEvent
import com.intellij.openapi.roots.ModuleRootListener
import me.danwi.sqlex.idea.repositroy.sqlexRepositoryServices
import me.danwi.sqlex.idea.util.extension.modules
import me.danwi.sqlex.idea.util.extension.unmarkAllGeneratedSourceRoot

class SqlExModuleRootListener : ModuleRootListener {
    override fun rootsChanged(event: ModuleRootEvent) {
        event.project.modules.forEach { it.unmarkAllGeneratedSourceRoot() }
        event.project.sqlexRepositoryServices.forEach { it.syncSourceRoot() }
    }
}
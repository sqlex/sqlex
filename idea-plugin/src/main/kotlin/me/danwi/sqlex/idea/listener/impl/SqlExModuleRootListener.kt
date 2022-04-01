package me.danwi.sqlex.idea.listener.impl

import com.intellij.openapi.roots.ModuleRootEvent
import com.intellij.openapi.roots.ModuleRootListener
import me.danwi.sqlex.idea.service.syncRepositoryService
import me.danwi.sqlex.idea.util.extension.modules
import me.danwi.sqlex.idea.util.extension.unmarkAllGeneratedSourceRoot

class SqlExModuleRootListener : ModuleRootListener {
    override fun rootsChanged(event: ModuleRootEvent) {
        event.project.modules.forEach { it.unmarkAllGeneratedSourceRoot() }
        syncRepositoryService(event.project)
    }
}
package me.danwi.sqlex.idea.listener.impl

import com.intellij.openapi.roots.ModuleRootEvent
import com.intellij.openapi.roots.ModuleRootListener
import me.danwi.sqlex.idea.service.syncRepositoryService

class SqlExModuleRootListener : ModuleRootListener {
    override fun rootsChanged(event: ModuleRootEvent) {
        syncRepositoryService(event.project)
    }
}
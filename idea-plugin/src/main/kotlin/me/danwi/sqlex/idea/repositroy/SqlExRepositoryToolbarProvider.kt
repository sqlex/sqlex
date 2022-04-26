package me.danwi.sqlex.idea.repositroy

import com.intellij.ide.ActivityTracker
import com.intellij.openapi.Disposable
import com.intellij.openapi.editor.toolbar.floating.AbstractFloatingToolbarProvider
import com.intellij.openapi.editor.toolbar.floating.FloatingToolbarComponent

class SqlExRepositoryToolbarProvider : AbstractFloatingToolbarProvider("me.danwi.sqlex.FloatingGroup"),
    SqlExRepositoryEventListener {
    override val autoHideable = true

    override fun created(newRepositoryService: SqlExRepositoryService) {
        ActivityTracker.getInstance().inc()
    }

    override fun removed(oldRepositoryService: SqlExRepositoryService) {
        ActivityTracker.getInstance().inc()
    }

    override fun beforeRefresh(repositoryService: SqlExRepositoryService) {
        ActivityTracker.getInstance().inc()
    }

    override fun afterRefresh(repositoryService: SqlExRepositoryService) {
        ActivityTracker.getInstance().inc()
    }

    override fun validChanged(repositoryService: SqlExRepositoryService, isValid: Boolean) {
        ActivityTracker.getInstance().inc()
    }

    override fun register(component: FloatingToolbarComponent, parentDisposable: Disposable) {
        super.register(component, parentDisposable)
        component.scheduleShow()
    }
}
package me.danwi.sqlex.idea.editor

import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.ui.EditorNotificationPanel
import com.intellij.ui.EditorNotifications
import me.danwi.sqlex.idea.listener.SqlExRepositoryEventListener
import me.danwi.sqlex.idea.repositroy.SqlExRepositoryService
import me.danwi.sqlex.idea.util.extension.isSqlExConfig
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService

private val Key = Key<EditorNotificationPanel>("me.danwi.sqlex.refresh.notification")

class SqlExRefreshNotificationProvider(private val project: Project) :
    EditorNotifications.Provider<EditorNotificationPanel>(), SqlExRepositoryEventListener {
    override fun getKey(): Key<EditorNotificationPanel> {
        return Key
    }

    override fun created(newRepositoryService: SqlExRepositoryService) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun removed(oldRepositoryService: SqlExRepositoryService) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun beforeRefresh(repositoryService: SqlExRepositoryService) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun afterRefresh(repositoryService: SqlExRepositoryService) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }

    override fun validChanged(repositoryService: SqlExRepositoryService, isValid: Boolean) {
        EditorNotifications.getInstance(project).updateAllNotifications()
    }
    override fun createNotificationPanel(file: VirtualFile, fileEditor: FileEditor): EditorNotificationPanel? {
        val sqlExRepositoryService = file.sqlexRepositoryService ?: return null

        if (!file.isSqlExConfig && sqlExRepositoryService.isValid)
            return null

        val panel = EditorNotificationPanel()

        if (sqlExRepositoryService.isRefreshing) {
            panel.text = "SqlEx: 正在重建索引..."
        } else if (!sqlExRepositoryService.isValid) {
            panel.text = "SqlEx: 配置或者Schema已经过时, 请重建索引"
        } else {
            panel.text = "SqlEx: Schema成功同步"
        }

        if (sqlExRepositoryService.isRefreshing) {
            panel.createActionLabel("停止索引") {
                file.sqlexRepositoryService?.stopRefresh()
            }
        } else if (!sqlExRepositoryService.isValid) {
            panel.createActionLabel("重建索引") {
                file.sqlexRepositoryService?.refresh()
            }
        } else if (file.isSqlExConfig) {
            panel.createActionLabel("强制重建索引") {
                file.sqlexRepositoryService?.refresh()
            }
        }

        return panel
    }
}
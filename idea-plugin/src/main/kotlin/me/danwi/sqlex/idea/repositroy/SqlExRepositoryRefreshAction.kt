package me.danwi.sqlex.idea.repositroy

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import icons.SqlExIcons
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService

class SqlExRepositoryRefreshAction : AnAction("索引已经过期,点击刷新SqlEx索引", null, SqlExIcons.RefreshAction) {
    override fun actionPerformed(event: AnActionEvent) {
        val virtualFile = event.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        virtualFile.sqlexRepositoryService?.refresh()
    }

    override fun update(event: AnActionEvent) {
        val virtualFile = event.getData(CommonDataKeys.VIRTUAL_FILE)
        if (virtualFile == null) {
            event.presentation.isEnabledAndVisible = false
            return
        }
        event.presentation.isEnabledAndVisible = virtualFile.sqlexRepositoryService?.isValid == false
    }
}
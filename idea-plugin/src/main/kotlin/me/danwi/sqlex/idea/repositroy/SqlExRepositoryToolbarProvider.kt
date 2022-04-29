package me.danwi.sqlex.idea.repositroy

import com.intellij.openapi.Disposable
import com.intellij.openapi.actionSystem.*
import com.intellij.openapi.editor.toolbar.floating.AbstractFloatingToolbarProvider
import com.intellij.openapi.editor.toolbar.floating.FloatingToolbarComponent
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.VirtualFile
import icons.SqlExIcons
import me.danwi.sqlex.idea.util.extension.isInSqlExSourceRoot
import me.danwi.sqlex.idea.util.extension.sourceRoot
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService

abstract class SqlExRepositoryBaseProvider(groupId: String) : AbstractFloatingToolbarProvider(groupId) {
    override val autoHideable = false
    override fun register(dataContext: DataContext, component: FloatingToolbarComponent, parentDisposable: Disposable) {
        super.register(dataContext, component, parentDisposable)
        val project = dataContext.getData(CommonDataKeys.PROJECT) ?: return
        val document = dataContext.getData(CommonDataKeys.EDITOR)?.document ?: return
        val file = FileDocumentManager.getInstance().getFile(document) ?: return

        val connect = project.messageBus.connect()
        connect.subscribe(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC, object : SqlExRepositoryEventListener {
            override fun created(newRepositoryService: SqlExRepositoryService) {
                update(file, component)
            }

            override fun removed(oldRepositoryService: SqlExRepositoryService) {
                update(file, component)
            }

            override fun beforeRefresh(repositoryService: SqlExRepositoryService) {
                update(file, component)
            }

            override fun afterRefresh(repositoryService: SqlExRepositoryService) {
                update(file, component)
            }

            override fun validChanged(repositoryService: SqlExRepositoryService, isValid: Boolean) {
                update(file, component)
            }
        })
        Disposer.register(parentDisposable, connect)
        update(file, component)
    }

    fun update(file: VirtualFile, component: FloatingToolbarComponent) {
        if (isVisible(file))
            component.scheduleShow()
        else
            component.scheduleHide()
    }

    abstract fun isVisible(file: VirtualFile): Boolean
}

class SqlExRepositoryImportProvider : SqlExRepositoryBaseProvider("me.danwi.sqlex.ImportFloatingToolbarGroup") {
    override fun isVisible(file: VirtualFile) = file.isInSqlExSourceRoot && file.sqlexRepositoryService == null

    override val actionGroup: ActionGroup
        get() {
            val actions = DefaultActionGroup()
            //Repository导入
            actions.add(object : AnAction("该文件所处的SqlEx Repository尚未被导入,点击立刻导入", null, SqlExIcons.ImportAction) {
                override fun actionPerformed(event: AnActionEvent) {
                    val virtualFile = event.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
                    val sourceRoot = virtualFile.sourceRoot ?: return
                    event.project?.importRepository(sourceRoot)
                }
            })
            return actions
        }
}

class SqlExRepositoryRefreshProvider : SqlExRepositoryBaseProvider("me.danwi.sqlex.RefreshFloatingToolbarGroup") {
    override fun isVisible(file: VirtualFile) = file.sqlexRepositoryService?.isValid == false

    override val actionGroup: ActionGroup
        get() {
            val actions = DefaultActionGroup()
            //索引更新
            actions.add(object : AnAction("索引已经过期,点击刷新SqlEx索引", null, SqlExIcons.RefreshAction) {
                override fun actionPerformed(event: AnActionEvent) {
                    val virtualFile = event.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
                    virtualFile.sqlexRepositoryService?.refresh()
                }
            })
            return actions
        }
}
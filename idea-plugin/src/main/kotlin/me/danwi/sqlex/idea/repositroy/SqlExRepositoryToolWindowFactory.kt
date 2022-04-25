package me.danwi.sqlex.idea.repositroy

import com.intellij.icons.AllIcons
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.DefaultActionGroup
import com.intellij.openapi.fileChooser.FileChooser
import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.SimpleToolWindowPanel
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.OnePixelSplitter
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBList
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.content.ContentFactory
import com.intellij.util.ui.UIUtil
import com.intellij.util.ui.components.BorderLayoutPanel
import me.danwi.sqlex.idea.config.SqlExConfigFileType
import me.danwi.sqlex.idea.listener.SqlExRepositoryEventListener
import me.danwi.sqlex.idea.util.extension.projectRootRelativePath
import javax.swing.BorderFactory
import javax.swing.JComponent
import javax.swing.SwingConstants

class SqlExRepositoryToolWindowFactory : ToolWindowFactory {
    private var project: Project? = null

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        this.project = project
        val repositoryToolWindow = SimpleToolWindowPanel(false, true)

        repositoryToolWindow.toolbar = toolbar(repositoryToolWindow)
        repositoryToolWindow.setContent(OnePixelSplitter(false).apply {
            proportion = 0.2F
            firstComponent = repositoriesView
            secondComponent = consoleView
        })

        val content = ContentFactory.SERVICE.getInstance().createContent(repositoryToolWindow, "", false)
        toolWindow.contentManager.addContent(content)

        val connect = project.messageBus.connect()
        connect
            .subscribe(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC, object : SqlExRepositoryEventListener {
                override fun created(newRepositoryService: SqlExRepositoryService) {
                    sync()
                }

                override fun removed(oldRepositoryService: SqlExRepositoryService) {
                    sync()
                }

                override fun beforeRefresh(repositoryService: SqlExRepositoryService) {
                    sync()
                }

                override fun afterRefresh(repositoryService: SqlExRepositoryService) {
                    sync()
                }

                override fun validChanged(repositoryService: SqlExRepositoryService, isValid: Boolean) {
                    sync()
                }
            })
        sync()
    }

    //同步
    private fun sync() {
        val project = project ?: return
        repositoryListView.setListData(project.sqlexRepositoryServices.toTypedArray())
    }

    private var currentSelectRepositoryService: SqlExRepositoryService? = null

    //工具栏
    private fun toolbar(target: JComponent): JComponent {
        val actionManager = ActionManager.getInstance()

        val actions = DefaultActionGroup()
        actions.add(object : AnAction(AllIcons.Actions.Refresh) {
            override fun actionPerformed(event: AnActionEvent) {
                project?.syncRepositoryService()
            }
        })
        actions.addSeparator()
        actions.add(object : AnAction(AllIcons.General.Add) {
            override fun actionPerformed(event: AnActionEvent) {
                val chooseConfigFile = FileChooser.chooseFile(
                    FileChooserDescriptorFactory.createSingleFileDescriptor(SqlExConfigFileType.INSTANCE),
                    project, LocalFileSystem.getInstance().findFileByPath(project?.basePath ?: return)
                ) ?: return
                project?.importRepository(chooseConfigFile.parent)
            }
        })
        actions.add(object : AnAction(AllIcons.General.Remove) {
            override fun actionPerformed(event: AnActionEvent) {
                project?.removeImportedRepository(currentSelectRepositoryService?.sourceRoot ?: return)
            }

            override fun update(event: AnActionEvent) {
                event.presentation.isEnabled = currentSelectRepositoryService?.isRefreshing == false
            }
        })

        val actionToolbar = actionManager.createActionToolbar("SqlEx.Repository", actions, true)
        actionToolbar.setOrientation(SwingConstants.VERTICAL)
        actionToolbar.targetComponent = target

        return actionToolbar.component
    }

    //列表view
    private val repositoryListView by lazy {
        JBList<SqlExRepositoryService>().apply {
            //item渲染
            installCellRenderer {
                BorderLayoutPanel().apply {
                    addToLeft(
                        JBLabel(
                            if (it.isValid)
                                AllIcons.General.InspectionsOK
                            else if (it.isRefreshing)
                                AllIcons.Actions.BuildLoadChanges
                            else
                                AllIcons.General.Error
                        ).apply {
                            border = BorderFactory.createEmptyBorder(0, 4, 0, 4)
                        }
                    )
                    addToCenter(
                        JBLabel(if (it.isRefreshing) "正在解析" else it.rootPackage).apply {
                            border = BorderFactory.createEmptyBorder(0, 2, 0, 2)
                            componentStyle = UIUtil.ComponentStyle.LARGE
                            fontColor = UIUtil.FontColor.BRIGHTER
                        }
                    )
                    addToRight(
                        JBLabel(it.sourceRoot.projectRootRelativePath ?: "未知目录").apply {
                            border = BorderFactory.createEmptyBorder(0, 2, 0, 2)
                            componentStyle = UIUtil.ComponentStyle.SMALL
                        }
                    )
                }
            }

            //item选择事件
            addListSelectionListener {
                currentSelectRepositoryService = this.selectedValue
            }
        }
    }

    //repository列表
    private val repositoriesView by lazy {
        BorderLayoutPanel().apply {
            add(JBScrollPane(repositoryListView))
        }
    }

    //控制台
    private val consoleView by lazy {
        BorderLayoutPanel().apply {

        }
    }
}
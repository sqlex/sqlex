package me.danwi.sqlex.idea.repositroy

import com.intellij.execution.filters.TextConsoleBuilderFactory
import com.intellij.execution.ui.ConsoleViewContentType
import com.intellij.icons.AllIcons
import com.intellij.notification.NotificationType
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
import me.danwi.sqlex.idea.util.extension.projectRootRelativePath
import me.danwi.sqlex.idea.util.extension.showNotification
import javax.swing.BorderFactory
import javax.swing.JComponent
import javax.swing.SwingConstants

class SqlExRepositoryToolWindowFactory : ToolWindowFactory {

    private lateinit var project: Project

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        this.project = project
        val repositoryToolWindow = SimpleToolWindowPanel(false, true)

        repositoryToolWindow.toolbar = toolbar(repositoryToolWindow)
        repositoryToolWindow.setContent(OnePixelSplitter(false).apply {
            proportion = 0.2F
            firstComponent = repositoriesView
            secondComponent = BorderLayoutPanel().apply { add(consoleView.component) }
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

                override fun clearOutput(repositoryService: SqlExRepositoryService) {
                    if (currentSelectRepositoryService == repositoryService)
                        consoleView.clear()
                }

                override fun output(
                    repositoryService: SqlExRepositoryService,
                    text: String, type: ConsoleViewContentType
                ) {
                    if (currentSelectRepositoryService == repositoryService)
                        consoleView.print(text + "\n", type)
                    //如果是错误输出,则选择对应的console输出来显示
                    if (type == ConsoleViewContentType.ERROR_OUTPUT)
                        repositoryListView.setSelectedValue(repositoryService, true)
                }
            })
        sync()
    }

    //同步
    private fun sync() {
        val services = project.sqlexRepositoryServices
        repositoryListView.setListData(services.toTypedArray())
        if (currentSelectRepositoryService == null && services.isNotEmpty())
            repositoryListView.setSelectedValue(services[0], true)
    }

    private var currentSelectRepositoryService: SqlExRepositoryService? = null
        set(value) {
            field = value
            consoleView.clear()
            if (value == null)
                return
            value.outputs.forEach { consoleView.print(it.first + "\n", it.second) }
        }

    //工具栏
    private fun toolbar(target: JComponent): JComponent {
        val actionManager = ActionManager.getInstance()

        val actions = DefaultActionGroup()
        actions.add(object : AnAction(AllIcons.Actions.Refresh) {
            override fun actionPerformed(event: AnActionEvent) {
                project.syncRepositoryService()
            }
        })
        actions.addSeparator()
        actions.add(object : AnAction(AllIcons.General.Add) {
            override fun actionPerformed(event: AnActionEvent) {
                try {
                    val chooseConfigFile = FileChooser.chooseFile(
                        FileChooserDescriptorFactory.createSingleFileDescriptor(SqlExConfigFileType.INSTANCE),
                        project, LocalFileSystem.getInstance().findFileByPath(project.basePath ?: return)
                    ) ?: return
                    project.importRepository(chooseConfigFile.parent)
                } catch (e: Exception) {
                    e.message?.let { event.project?.showNotification(it, NotificationType.WARNING) }
                }
            }
        })
        actions.add(object : AnAction(AllIcons.General.Remove) {
            override fun actionPerformed(event: AnActionEvent) {
                project.removeImportedRepository(currentSelectRepositoryService?.sourceRoot ?: return)
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

                    border = BorderFactory.createEmptyBorder(3, 5, 3, 5)
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
        TextConsoleBuilderFactory.getInstance().createBuilder(project).console
    }
}

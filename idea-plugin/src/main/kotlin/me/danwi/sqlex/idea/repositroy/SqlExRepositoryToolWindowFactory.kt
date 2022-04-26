package me.danwi.sqlex.idea.repositroy

import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory

class SqlExRepositoryToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val repositoryToolWindow = SqlExRepositoryToolWindow(project)
        val toolWindowContent = repositoryToolWindow.initAndGetContent()
        val content = ContentFactory.SERVICE.getInstance().createContent(toolWindowContent, "", false)
        toolWindow.contentManager.addContent(content)
        Disposer.register(content, repositoryToolWindow)
    }
}

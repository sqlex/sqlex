package me.danwi.sqlex.idea.sqlm

import com.intellij.database.console.JdbcConsoleProvider
import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.ui.EditorNotificationPanel
import com.intellij.ui.EditorNotificationProvider
import me.danwi.sqlex.idea.sqls.SqlExSchemaFileType
import me.danwi.sqlex.idea.util.extension.findDataSource
import me.danwi.sqlex.idea.util.extension.removeDataSource
import me.danwi.sqlex.idea.util.extension.sourceRoot
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import java.util.function.Function
import javax.swing.JComponent

class SqlExMethodDebugModeNotificationProvider : EditorNotificationProvider {
    override fun collectNotificationData(
        project: Project,
        file: VirtualFile
    ): Function<in FileEditor, out JComponent?> {
        return Function {
            //判断是不是sqls/sqlm文件
            if (file.fileType != SqlExSchemaFileType.INSTANCE && file.fileType != SqlExMethodFileType.INSTANCE)
                return@Function null
            //获取jdbc console
            val console = JdbcConsoleProvider.getConsole(project, file) ?: return@Function null
            val panel = EditorNotificationPanel()
            panel.text = "SqlEx: 当前文件处于调试模式,智能提示将参照真实数据库结构,请注意结构差异"
            panel.createActionLabel("关闭调试模式") {
                //销毁console
                Disposer.dispose(console)
                //刷新repository重新设置解析路径
                val sourceRoot = file.sourceRoot ?: return@createActionLabel
                val dataSource = project.findDataSource(sourceRoot) ?: return@createActionLabel
                project.removeDataSource(dataSource, sourceRoot)
                file.sqlexRepositoryService?.refresh()
            }
            return@Function panel
        }
    }
}
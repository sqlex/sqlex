package me.danwi.sqlex.idea.listener.impl

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.*
import me.danwi.sqlex.idea.sqlm.SqlExMethodFileType
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.util.invokeWithoutException
import me.danwi.sqlex.parser.util.isSqlExMethodFilePath

class SqlExMethodChangeListener(private val project: Project) : BulkFileListener {
    override fun after(events: MutableList<out VFileEvent>) {
        //添加
        events.filterIsInstance<VFileCreateEvent>()
            .filter { it.file?.fileType is SqlExMethodFileType }
            .forEach { invokeWithoutException { it.file?.sqlexRepositoryService?.updateSqlMethodFile(it.file) } }

        //删除
        events.filterIsInstance<VFileDeleteEvent>()
            .filter { it.file.fileType is SqlExMethodFileType }
            .forEach { it.file.parent.sqlexRepositoryService?.removeSqlMethodFile(it.file) }

        //内容变更
        events.filterIsInstance<VFileContentChangeEvent>()
            .filter { it.file.fileType is SqlExMethodFileType }
            .forEach { invokeWithoutException { it.file.sqlexRepositoryService?.updateSqlMethodFile(it.file) } }

        //移动事件
        events.filterIsInstance<VFileMoveEvent>()
            .filter { it.oldPath.isSqlExMethodFilePath && it.newPath.isSqlExMethodFilePath }
            .forEach {
                invokeWithoutException {
                    it.oldParent.sqlexRepositoryService?.removeSqlMethodFile(it.oldPath)
                    it.newParent.sqlexRepositoryService?.updateSqlMethodFile(it.file)
                }
            }

        //拷贝事件
        events.filterIsInstance<VFileCopyEvent>()
            .filter { it.newChildName.isSqlExMethodFilePath }
            .forEach {
                val newFile = it.newParent.findChild(it.newChildName) ?: throw Exception("无法找到拷贝的目标文件")
                if (newFile.fileType is SqlExMethodFileType)
                    invokeWithoutException { it.file.sqlexRepositoryService?.updateSqlMethodFile(newFile) }
            }

        //文件属性修改事件
        events.filterIsInstance<VFilePropertyChangeEvent>()
            .filter { it.file.fileType is SqlExMethodFileType }
            .forEach {
                invokeWithoutException {
                    it.file.sqlexRepositoryService?.removeSqlMethodFile(it.oldPath)
                    it.file.sqlexRepositoryService?.updateSqlMethodFile(it.file)
                }
            }
    }
}
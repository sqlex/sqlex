package me.danwi.sqlex.idea.listener.impl

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileContentChangeEvent
import com.intellij.openapi.vfs.newvfs.events.VFileCopyEvent
import com.intellij.openapi.vfs.newvfs.events.VFileCreateEvent
import com.intellij.openapi.vfs.newvfs.events.VFileDeleteEvent
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.openapi.vfs.newvfs.events.VFileMoveEvent
import com.intellij.openapi.vfs.newvfs.events.VFilePropertyChangeEvent
import me.danwi.sqlex.idea.service.syncRepositoryService
import me.danwi.sqlex.idea.util.extension.isSqlExConfig
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.util.isSqlExConfigFileName
import me.danwi.sqlex.parser.util.isSqlExConfigFilePath

class SqlExConfigChangeListener(private val project: Project) : BulkFileListener {
    override fun after(events: MutableList<out VFileEvent>) {
        //过滤出所有需要同步sync repository service的事件
        events.forEach {
            if (
                (it is VFileCreateEvent && it.file.isSqlExConfig)
                || (it is VFileCopyEvent && it.newChildName.isSqlExConfigFileName)
                || (it is VFileMoveEvent && (it.newPath.isSqlExConfigFilePath || it.oldPath.isSqlExConfigFilePath))
                || (it is VFilePropertyChangeEvent && (it.newPath.isSqlExConfigFilePath || it.oldPath.isSqlExConfigFilePath))
                || (it is VFileDeleteEvent && it.file.isSqlExConfig)
            ) {
                syncRepositoryService(project)
            }
        }

        //如果Config的内容发生了变动
        events.filter { it is VFileContentChangeEvent && it.file.isSqlExConfig }
            .forEach {
                val service = it.file?.sqlexRepositoryService ?: return@forEach
                service.isValid = false
                if (service.autoRefresh)
                    service.refresh()
            }
    }
}

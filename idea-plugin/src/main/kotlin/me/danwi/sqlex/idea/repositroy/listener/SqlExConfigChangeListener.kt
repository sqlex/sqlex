package me.danwi.sqlex.idea.repositroy.listener

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.*
import me.danwi.sqlex.idea.repositroy.showMaybeSqlExImportNotification
import me.danwi.sqlex.idea.util.extension.isSqlExConfig
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService

class SqlExConfigChangeListener(private val project: Project) : BulkFileListener {
    override fun after(events: MutableList<out VFileEvent>) {
        //如果Config的内容发生了变动
        events.filter { it is VFileContentChangeEvent && it.file.isSqlExConfig }
            .forEach { it.file?.sqlexRepositoryService?.isValid = false }

        //如果新增了Config
        if (events.any { it is VFileCreateEvent && it.file.isSqlExConfig })
            project.showMaybeSqlExImportNotification()
    }
}

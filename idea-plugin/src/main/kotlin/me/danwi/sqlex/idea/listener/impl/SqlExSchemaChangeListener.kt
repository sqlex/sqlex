package me.danwi.sqlex.idea.listener.impl

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.*
import me.danwi.sqlex.idea.util.extension.isSqlExSchema
import me.danwi.sqlex.idea.util.extension.sqlexRepositoryService
import me.danwi.sqlex.parser.util.isSqlExSchemaFilePath

class SqlExSchemaChangeListener(private val project: Project) : BulkFileListener {
    override fun after(events: MutableList<out VFileEvent>) {
        events.filter {
            (it is VFileCopyEvent && it.newChildName.isSqlExSchemaFilePath)
                    || (it is VFileMoveEvent && (it.newPath.isSqlExSchemaFilePath || it.oldPath.isSqlExSchemaFilePath))
                    || (it is VFilePropertyChangeEvent && (it.newPath.isSqlExSchemaFilePath || it.oldPath.isSqlExSchemaFilePath))
                    || (it is VFileDeleteEvent && it.path.isSqlExSchemaFilePath)
                    || (it is VFileContentChangeEvent && it.file.isSqlExSchema)
        }.filter {
            if (it is VFilePropertyChangeEvent) {
                if (it.propertyName == "name")
                    return@filter it.newValue != it.oldValue
            }
            return@filter true
        }.forEach {
            val service = it.file?.sqlexRepositoryService ?: return@forEach
            service.isValid = false
            if (service.autoRefresh)
                service.refresh()
        }
    }
}
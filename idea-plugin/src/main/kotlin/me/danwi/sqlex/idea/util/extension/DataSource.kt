package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.sql.database.SqlDataSourceImpl
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import java.nio.file.Paths

//添加Database Tool数据源的名称前缀
const val SQLEX_DATABASE_TOOL_NAME_PREFIX = "SqlEx:"

//数据源source root属性key
private val dataSourceSourceRootPropertyKey = PropertyKey<String>("me.danwi.sqlex.datatools.datasource.sourceroot")

var SqlDataSourceImpl.sqlexName: String
    get() = this.name.removePrefix("$SQLEX_DATABASE_TOOL_NAME_PREFIX ")
    set(value) {
        this.name = "$SQLEX_DATABASE_TOOL_NAME_PREFIX $value"
    }

fun SqlDataSourceImpl.getSqlExSourceRootPath(project: Project): String? {
    return project.getProperty(dataSourceSourceRootPropertyKey.child(this.uniqueId))
}

fun SqlDataSourceImpl.setSqlExSourceRootPath(project: Project, path: String?) {
    val propertyKey = dataSourceSourceRootPropertyKey.child(this.uniqueId)
    if (path == null)
        project.unsetProperty(propertyKey)
    else
        project.setProperty(propertyKey, path)
}

fun SqlDataSourceImpl.setDDLFile(project: Project, file: VirtualFile?) {
    invokeAndWaitIfNeeded {
        //建立映射
        SqlDialectMappings.getInstance(project).setMapping(file, MysqlDialect.INSTANCE)
        this.setFiles(arrayOf(file))
    }
}

fun SqlDataSourceImpl.setDDL(project: Project, ddl: String?) {
    invokeAndWaitIfNeeded {
        val sqlDialectMappings = SqlDialectMappings.getInstance(project)
        //删除旧的文件
        sqlFiles.forEach {
            sqlDialectMappings.setMapping(it.virtualFile, null)
            runWriteAction { it.virtualFile.delete(this) }
        }
        this.setDDLFile(
            project, createVirtualFile(
                Paths.get(project.pluginTempDir.toString(), "ddls", uniqueId, "${sqlexName}.sql").toString(),
                ddl ?: ""
            )
        )
    }
}
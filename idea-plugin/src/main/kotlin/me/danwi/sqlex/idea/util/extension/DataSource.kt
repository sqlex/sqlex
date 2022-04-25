package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.application.invokeAndWaitIfNeeded
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

var SqlDataSourceImpl.sqlexSourceRootPath: String?
    get() = project.getProperty(dataSourceSourceRootPropertyKey.child(this.uniqueId))
    set(value) {
        val propertyKey = dataSourceSourceRootPropertyKey.child(this.uniqueId)
        if (value == null)
            project.unsetProperty(propertyKey)
        else
            project.setProperty(propertyKey, value)
    }

var SqlDataSourceImpl.ddlFile: VirtualFile?
    get() = this.sqlFiles.map { it.virtualFile }.firstOrNull()
    set(value) {
        invokeAndWaitIfNeeded {
            //建立映射
            SqlDialectMappings.getInstance(project).setMapping(value, MysqlDialect.INSTANCE)
            this.setFiles(arrayOf(value))
        }
    }

var SqlDataSourceImpl.ddl: String?
    get() = this.ddlFile?.textContent
    set(value) {
        invokeAndWaitIfNeeded {
            val sqlDialectMappings = SqlDialectMappings.getInstance(project)
            //删除旧的文件
            sqlFiles.forEach {
                sqlDialectMappings.setMapping(it.virtualFile, null)
                runWriteAction { it.virtualFile.delete(this) }
            }
            this.ddlFile = createVirtualFile(
                Paths.get(project.pluginTempDir.toString(), "ddls", uniqueId, "${sqlexName}.sql").toString(),
                value ?: ""
            )
        }
    }
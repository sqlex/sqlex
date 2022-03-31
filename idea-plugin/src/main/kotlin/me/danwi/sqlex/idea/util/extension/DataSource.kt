package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.application.runWriteAction
import com.intellij.sql.database.SqlDataSourceImpl
import com.intellij.sql.database.SqlDataSourceManager
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import java.nio.file.Paths

var SqlDataSourceImpl.sqlexName: String
    get() = this.name.removePrefix("$SQLEX_DATABASE_TOOL_PREFIX ")
    set(value) {
        invokeAndWaitIfNeeded {
            SqlDataSourceManager.getInstance(this.project)
                .renameDataSource(this, "$SQLEX_DATABASE_TOOL_PREFIX $value")
        }
    }

var SqlDataSourceImpl.ddl: String?
    get() = this.sqlFiles.map { it.virtualFile.textContent }.firstOrNull()
    set(value) {
        invokeAndWaitIfNeeded {
            val sqlDialectMappings = SqlDialectMappings.getInstance(project)
            //删除旧的文件
            sqlFiles.forEach {
                sqlDialectMappings.setMapping(it.virtualFile, null)
                runWriteAction { it.virtualFile.delete(this) }
            }
            //写入新的文件
            val ddlVirtualFile =
                createVirtualFile(
                    Paths.get(project.pluginTempDir.toString(), "ddls", "${sqlexName}.sql").toString(),
                    value ?: ""
                )
            //建立映射
            sqlDialectMappings.setMapping(ddlVirtualFile, MysqlDialect.INSTANCE)
            this.setFiles(arrayOf(ddlVirtualFile))
        }
    }
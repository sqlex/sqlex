package me.danwi.sqlex.idea.util.extension

import com.intellij.database.util.TreePattern
import com.intellij.database.util.TreePatternUtils
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.application.PathManager
import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.application.runWriteAction
import com.intellij.openapi.module.Module
import com.intellij.openapi.module.ModuleManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.roots.ModuleRootManager
import com.intellij.openapi.roots.ModuleRootModificationUtil
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.sql.database.SqlDataSourceImpl
import com.intellij.sql.database.SqlDataSourceManager
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.SqlImportUtil
import com.intellij.sql.dialects.SqlResolveMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import me.danwi.sqlex.idea.service.SqlExRepositoryService
import me.danwi.sqlex.parser.util.SqlExGeneratedTagFileName
import java.nio.file.Path
import java.nio.file.Paths

//获取项目的模块
val Project.modules: Array<Module>
    inline get() = ModuleManager.getInstance(this).modules

//获取project的临时目录
val Project.pluginTempDir: Path
    inline get() = Paths.get(PathManager.getTempPath(), "sqlex", this.locationHash)

//获取项目下的SqlExRepositoryService列表
val Project.sqlexRepositoryServices: List<SqlExRepositoryService>
    inline get() = this.modules.flatMap { it.sqlexRepositoryServices }

//添加Database Tool数据源
const val SQLEX_DATABASE_TOOL_PREFIX = "SqlEx:"

val Project.allSqlExDataSources: List<SqlDataSourceImpl>
    get() = SqlDataSourceManager.getInstance(this).dataSources.filter { it.name.startsWith(SQLEX_DATABASE_TOOL_PREFIX) }

fun Project.addDataSource(name: String, ddl: String, sourceRoot: VirtualFile): SqlDataSourceImpl {
    return invokeAndWaitIfNeeded {
        //获取数据源管理器
        val dataSourceManager = SqlDataSourceManager.getInstance(this)
        //创建数据源
        val dataSource = dataSourceManager.createEmpty()
        dataSource.name = "$SQLEX_DATABASE_TOOL_PREFIX $name"
        //创建ddl文件
        val ddlVirtualFile =
            createVirtualFile(Paths.get(this.pluginTempDir.toString(), "ddls", "$name.sql").toString(), ddl)
                ?: throw Exception("无法找到ddl对应的虚拟文件")
        //设置ddl文件方言
        SqlDialectMappings.getInstance(this).setMapping(ddlVirtualFile, MysqlDialect.INSTANCE)
        //设置ddl文件
        dataSource.setFiles(arrayOf(ddlVirtualFile))
        dataSourceManager.addDataSource(dataSource)
        //设置解析路径
        SqlResolveMappings.getInstance(this)
            .setMapping(
                sourceRoot, TreePattern(
                    TreePatternUtils.create(
                        SqlImportUtil.getDataSourceName(dataSource),
                        SqlImportUtil.DATA_SOURCE
                    )
                )
            )
        dataSource
    }
}

fun Project.removeDataSource(dataSource: SqlDataSourceImpl, sourceRoot: VirtualFile? = null) {
    invokeAndWaitIfNeeded {
        val dataSourceManager = SqlDataSourceManager.getInstance(this)
        val sqlDialectMappings = SqlDialectMappings.getInstance(this)
        //删除文件
        dataSource.sqlFiles.forEach {
            sqlDialectMappings.setMapping(it.virtualFile, null)
            runWriteAction { it.virtualFile.delete(this) }
        }
        //设置解析路径
        if (sourceRoot != null)
            SqlResolveMappings.getInstance(this).setMapping(sourceRoot, null)
        dataSourceManager.removeDataSource(dataSource)
    }
}

fun Project.findDataSource(name: String): SqlDataSourceImpl? {
    return this.allSqlExDataSources.find { it.sqlexName == name }
}

//创建通知
fun Project.showNotification(text: String, type: NotificationType) {
    NotificationGroupManager.getInstance()
        .getNotificationGroup("SqlEx Notification Group")
        .createNotification(text, type)
        .notify(this)
}

//缓存key
val repositoryServiceCacheKey = Key<MutableList<SqlExRepositoryService>>("me.danwi.sqlex.repositoryServiceCaches")

//获取模块下的SqlExRepositoryService列表
val Module.sqlexRepositoryServices: MutableList<SqlExRepositoryService>
    inline get() {
        synchronized(this) {
            var services = this.getUserData(repositoryServiceCacheKey)
            if (services == null) {
                services = mutableListOf()
                this.putUserData(repositoryServiceCacheKey, services)
            }
            return services
        }
    }

//获取模块的SourceRoots
val Module.sourceRoots: Array<VirtualFile>
    inline get() = ModuleRootManager.getInstance(this).sourceRoots

//把所有生成的源码目录的源码标记去掉
fun Module.unmarkAllGeneratedSourceRoot() {
    //生成的源码目录
    val generatedSourceRoots = this.sourceRoots.filter { it.findChild(SqlExGeneratedTagFileName) != null }

    generatedSourceRoots.forEach { generatedSourceRoot ->
        val module = generatedSourceRoot.module ?: return@forEach
        ModuleRootModificationUtil.updateModel(module) { model ->
            //删除相关的源码目录
            model.contentEntries.forEach { contentEntry ->
                contentEntry.sourceFolders
                    .filter { it.file == generatedSourceRoot }
                    .forEach { contentEntry.removeSourceFolder(it) }
            }
        }
    }
}
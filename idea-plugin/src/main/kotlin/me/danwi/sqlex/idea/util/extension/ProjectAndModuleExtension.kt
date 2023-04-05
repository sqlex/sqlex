package me.danwi.sqlex.idea.util.extension

import com.google.gson.Gson
import com.intellij.database.util.TreePattern
import com.intellij.database.util.TreePatternUtils
import com.intellij.ide.util.PropertiesComponent
import com.intellij.notification.Notification
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.module.Module
import com.intellij.openapi.module.ModuleManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.roots.ModuleRootManager
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.sql.database.SqlDataSourceImpl
import com.intellij.sql.database.SqlDataSourceManager
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.SqlImportUtil
import com.intellij.sql.dialects.SqlResolveMappings
import me.danwi.sqlex.parser.util.SqlExGeneratedTagFileName
import java.nio.file.Path
import java.nio.file.Paths

//获取项目的模块
val Project.modules: Array<Module>
    inline get() = this.getService(ModuleManager::class.java).modules

//获取基于项目的存储目录
val Project.storeDir: Path
    inline get() = Paths.get(this.basePath ?: throw Exception("无法获取项目根目录"), Project.DIRECTORY_STORE_FOLDER, "sqlex")

val Project.allSqlExDataSources: List<SqlDataSourceImpl>
    get() = SqlDataSourceManager.getInstance(this)
        .dataSources
        .filter { it.name.startsWith(SQLEX_DATABASE_TOOL_NAME_PREFIX) }

fun Project.addDataSource(name: String, sourceRoot: VirtualFile): SqlDataSourceImpl {
    return invokeAndWaitIfNeeded {
        //获取数据源管理器
        val dataSourceManager = SqlDataSourceManager.getInstance(this)
        //创建数据源
        val dataSource = SqlDataSourceImpl("", this, null)
        dataSource.sqlexName = name
        dataSource.comment = "SqlEx schema of [ ${sourceRoot.projectRootRelativePath} ]"
        dataSource.setSqlExSourceRootPath(
            this,
            sourceRoot.projectRootRelativePath ?: throw Exception("无法获取source root相对路径")
        )
        dataSource.setDDL(this, "")  //暂时以空白作为内容
        //添加数据源
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
            runWriteAction {
                it.virtualFile.delete(this)
                //如果父目录为空,则删除,以免留下垃圾目录
                if (it.virtualFile.parent.children.isEmpty())
                    it.virtualFile.parent.delete(this)
            }
        }
        //设置解析路径
        if (sourceRoot != null)
            SqlResolveMappings.getInstance(this).setMapping(sourceRoot, null)
        //删除source root属性数据
        dataSource.setSqlExSourceRootPath(this, null)
        dataSourceManager.removeDataSource(dataSource)
    }
}

fun Project.findDataSource(sourceRoot: VirtualFile): SqlDataSourceImpl? {
    return this.allSqlExDataSources.find {
        it.getSqlExSourceRootPath(this) == sourceRoot.projectRootRelativePath
    }
}

//创建通知
fun Project.createNotification(text: String, type: NotificationType = NotificationType.INFORMATION): Notification {
    return NotificationGroupManager.getInstance()
        .getNotificationGroup("SqlEx Notification Group")
        .createNotification(text, type)
}

fun Project.showNotification(text: String, type: NotificationType = NotificationType.INFORMATION) {
    createNotification(text, type).notify(this)
}

//获取模块的SourceRoots
val Module.sourceRoots: Array<VirtualFile>
    inline get() = ModuleRootManager.getInstance(this).sourceRoots

//把所有生成的源码目录的源码标记去掉
fun Module.unmarkAllGeneratedSourceRoot() {
    this.sourceRoots
        .filter { it.findChild(SqlExGeneratedTagFileName) != null }
        .forEach { it.unmarkSource() }
}

//项目级别的属性存储
data class PropertyKey<T>(val key: String) {
    fun child(key: String): PropertyKey<T> {
        return PropertyKey<T>("${this.key}.$key")
    }
}

fun <T> Project.setProperty(key: PropertyKey<T>, value: T) {
    PropertiesComponent.getInstance(this).setValue(key.key, Gson().toJson(value))
}

inline fun <reified T> Project.getProperty(key: PropertyKey<T>): T? {
    val json = PropertiesComponent.getInstance(this).getValue(key.key)
    return Gson().fromJson(json, T::class.java)
}

fun <T> Project.unsetProperty(key: PropertyKey<T>) {
    PropertiesComponent.getInstance(this).unsetValue(key.key)
}
package me.danwi.sqlex.idea.repositroy

import com.intellij.execution.ui.ConsoleViewContentType
import com.intellij.notification.Notification
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.module.Module
import com.intellij.openapi.progress.ProcessCanceledException
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VfsUtilCore
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.psi.search.FileTypeIndex
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import me.danwi.sqlex.idea.config.SqlExConfigFileType
import me.danwi.sqlex.idea.util.extension.*
import me.danwi.sqlex.parser.RepositoryBuilder
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.util.SqlExConfigFileName
import me.danwi.sqlex.parser.util.schemaFileVersion
import java.nio.file.Paths
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock

//全局刷新锁
private val globalRepositoryServiceRefreshLocker = ReentrantLock()

class SqlExRepositoryService(val sourceRoot: VirtualFile) {
    // region 公有变量

    //是否正在刷新中
    val isRefreshing: Boolean
        get() = refreshLocker.isLocked

    //SqlEx是否过期
    var isValid = false
        set(value) {
            val oldValue = field
            field = value
            if (oldValue != value)
                messagePublisher.validChanged(this, value)
        }

    //SqlEx代码仓库
    var repository: SqlExRepository? = null
        private set

    //根包
    var rootPackage: String = ""

    //控制台输出
    val outputs: MutableList<Pair<String, ConsoleViewContentType>> = mutableListOf()
    // endregion

    // region 私有变量

    //对应的项目
    private val project: Project = sourceRoot.project ?: throw Exception("无法获取source root对应的project")

    //Repository事件消息publisher
    private val messagePublisher =
        project.messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC)

    //刷新任务锁
    private val refreshLocker = ReentrantLock()

    //任务指示器
    private var indicator: ProgressIndicator? = null

    //是否已经关闭
    private var isClosed = false
    //endregion

    init {
        //设置数据库方言
        invokeAndWaitIfNeeded {
            SqlDialectMappings.getInstance(project)
                .setMapping(sourceRoot, MysqlDialect.INSTANCE)
        }
    }

    private fun output(text: String, type: ConsoleViewContentType = ConsoleViewContentType.NORMAL_OUTPUT) {
        if (type == ConsoleViewContentType.ERROR_OUTPUT) {
            val toolWindow = ToolWindowManager.getInstance(project)
                .getToolWindow("SqlEx Repository") ?: return
            invokeLater { toolWindow.show() }
        }
        outputs.add(Pair(text, type))
        messagePublisher.output(this, text, type)
    }

    fun refresh() {
        synchronized(this) {
            if (isRefreshing) {
                //正在刷新中,取消任务
                stopRefresh()
            }
            //开启一个新的任务
            ProgressManager.getInstance()
                .run(object : Task.Backgroundable(
                    project,
                    "SqlEx 索引: ${this.sourceRoot.projectRootRelativePath}",
                    true
                ) {
                    override fun run(indicator: ProgressIndicator) {
                        refreshLocker.withLock {
                            //开始刷新
                            messagePublisher.beforeRefresh(this@SqlExRepositoryService)
                            //暴露任务指示器
                            this@SqlExRepositoryService.indicator = indicator
                            outputs.clear()
                            messagePublisher.clearOutput(this@SqlExRepositoryService)
                            indicator.isIndeterminate = true
                            indicator.text = "SqlEx: 等待任务开始"
                            output("准备开始建立索引")
                            //全局刷新锁
                            globalRepositoryServiceRefreshLocker.withLock {
                                //需要清理的builder
                                var builderToClean: RepositoryBuilder? = null
                                //需要清理的repository
                                var repositoryToClean: SqlExRepository? = null
                                try {
                                    indicator.checkCanceled()

                                    //解析配置
                                    indicator.isIndeterminate = true
                                    indicator.text = "SqlEx: 解析配置"
                                    val configFileContent =
                                        sourceRoot.configFile?.textContent ?: throw Exception("无法读取配置文件")
                                    val config = createSqlExConfig(configFileContent)
                                    rootPackage = config.rootPackage ?: throw Exception("配置文件中不存在根包信息")
                                    output("获取到根包信息: $rootPackage")
                                    indicator.checkCanceled()

                                    //扫描文件
                                    indicator.text = "SqlEx: 扫描文件"
                                    val schemaFiles = mutableListOf<VirtualFile>()
                                    val methodFiles = mutableListOf<VirtualFile>()
                                    VfsUtilCore.processFilesRecursively(sourceRoot) {
                                        indicator.checkCanceled()
                                        if (it.isSqlExSchema)
                                            schemaFiles.add(it)
                                        else if (it.isSqlExMethod)
                                            methodFiles.add(it)
                                        true
                                    }
                                    //排序
                                    val sortedSchemaFiles = schemaFiles
                                        .map { Pair(it.name.schemaFileVersion, it) }
                                        .filter { it.first !== null }
                                        .sortedBy { it.first }
                                        .map { it.second }

                                    indicator.checkCanceled()

                                    //解析schema文件
                                    indicator.text = "SqlEx: 解析Schema"
                                    indicator.isIndeterminate = false
                                    val builder = RepositoryBuilder(config)
                                    builderToClean = builder
                                    sortedSchemaFiles.forEachIndexed { index, file ->
                                        val relativePath =
                                            file.sourceRootRelativePath ?: throw Exception("无法获取${file.name}的相对路径")
                                        indicator.text =
                                            "SqlEx: 解析Schema(${index + 1}/${sortedSchemaFiles.size}) $relativePath"
                                        output("解析Schema: $relativePath")
                                        indicator.fraction = (index + 1) / sortedSchemaFiles.size.toDouble() / 2.0
                                        builder.addSchema(
                                            relativePath,
                                            file.textContent ?: throw Exception("无法读取${file.name}的文件内容")
                                        )
                                        indicator.checkCanceled()
                                    }
                                    indicator.isIndeterminate = true
                                    //构建SqlEx仓库
                                    val parserRepository = builder.build()
                                    val ddlScript = parserRepository.session.DDL
                                    val repository = SqlExRepository(project, parserRepository)
                                    repositoryToClean = repository
                                    this@SqlExRepositoryService.repository = repository

                                    indicator.checkCanceled()

                                    //同步DatabaseTools
                                    indicator.text = "SqlEx: 同步 Database Tools"
                                    output("同步 Database Tools")

                                    val rootPackage = config.rootPackage ?: throw Exception("无法获取SqlEx Config的根包名")

                                    //同步数据源
                                    //在现存的数据源中查找
                                    var dataSource = project.findDataSource(sourceRoot)
                                    //如果现有的数据源没有,则新建一个
                                    if (dataSource == null)
                                        dataSource = project.addDataSource(rootPackage, sourceRoot)
                                    //设置关键信息
                                    dataSource.sqlexName = rootPackage
                                    dataSource.setDDL(project, ddlScript)

                                    indicator.checkCanceled()

                                    //设置源码目录
                                    syncSourceRoot()

                                    //解析method文件
                                    indicator.text = "SqlEx: 解析Method"
                                    indicator.isIndeterminate = false
                                    methodFiles.forEachIndexed { index, file ->
                                        indicator.text =
                                            "SqlEx: 解析Method(${index + 1}/${methodFiles.size}) ${file.sourceRootRelativePath}"
                                        output("解析Method: ${file.sourceRootRelativePath}")
                                        indicator.fraction = (index + 1) / methodFiles.size.toDouble() / 2.0 + 0.5
                                        try {
                                            repository.updateMethodFile(file)
                                        } catch (e: SqlExRepositoryMethodException) {
                                            output(
                                                "\n解析Method文件 [${e.relativePath}] 错误:\n${e.message}\n",
                                                ConsoleViewContentType.ERROR_OUTPUT
                                            )
                                        }
                                        indicator.checkCanceled()
                                    }

                                    output("索引建立完成")
                                    isValid = true
                                } catch (e: Exception) {
                                    //清理builder
                                    builderToClean?.close()
                                    //清理repository
                                    repositoryToClean?.close()
                                    when (e) {
                                        is ProcessCanceledException -> {
                                            output("\nSqlEx索引更新被取消", ConsoleViewContentType.LOG_WARNING_OUTPUT)
                                        }
                                        is SqlExRepositorySchemaException -> {
                                            output(
                                                "\n解析Schema文件 [${e.relativePath}] 错误:\n${e.message}",
                                                ConsoleViewContentType.ERROR_OUTPUT
                                            )
                                        }
                                        else -> {
                                            output(
                                                "\n重建SqlEx索引时发生错误,索引构建失败:\n${e.message ?: e::class.java.simpleName}",
                                                ConsoleViewContentType.ERROR_OUTPUT
                                            )
                                        }
                                    }
                                } finally {
                                    this@SqlExRepositoryService.indicator = null
                                }
                            }
                            //完成刷新
                            messagePublisher.afterRefresh(this@SqlExRepositoryService)
                        }
                    }
                })
        }
    }

    fun stopRefresh() {
        this.indicator?.cancel()
    }

    fun syncSourceRoot() {
        synchronized(this) {
            if (isClosed)
                this.sourceRoot.unmarkSource()
            else
                this.sourceRoot.markAsSource()
        }
    }

    fun close() {
        synchronized(this) {
            if (isClosed)
                return
            isClosed = true
        }
        stopRefresh()
        syncSourceRoot()
        val dataSource = project.findDataSource(sourceRoot)
        if (dataSource != null)
            project.removeDataSource(dataSource, sourceRoot)
        this.repository?.close()
        this.repository = null
    }
}

//同步锁
private val repositoryServiceSyncLocker = ReentrantLock()

//缓存key
private val repositoryServiceCacheKey =
    Key<MutableList<SqlExRepositoryService>>("me.danwi.sqlex.RepositoryServiceCaches")

//配置key
private val importedRepositoriesPropertyKey = PropertyKey<List<String>>("me.danwi.sqlex.ImportedRepositories")
private val ignoredRepositoriesPropertyKey = PropertyKey<List<String>>("me.danwi.sqlex.IgnoredRepositories")

//获取模块下的SqlExRepositoryService列表
val Module.sqlexRepositoryServices: MutableList<SqlExRepositoryService>
    get() {
        synchronized(this) {
            var services = this.getUserData(repositoryServiceCacheKey)
            if (services == null) {
                services = mutableListOf()
                this.putUserData(repositoryServiceCacheKey, services)
            }
            return services
        }
    }

//获取项目下的SqlExRepositoryService列表
val Project.sqlexRepositoryServices: List<SqlExRepositoryService>
    inline get() = this.modules.flatMap { it.sqlexRepositoryServices }

//获取项目下的所有可能的SqlEx Source Root
val Project.allMaybeSqlExSourceRoot: List<VirtualFile>
    inline get() = FileTypeIndex
        .getFiles(SqlExConfigFileType.INSTANCE, GlobalSearchScope.allScope(this))
        .mapNotNull { it.parent }

//导入通知缓存key
private val importNotificationKey = Key<MutableList<Notification>>("me.danwi.sqlex.ImportNotification")

//显示可能存在的SqlEx Repository导入通知
fun Project.showMaybeSqlExImportNotification() {
    runInThread {
        runReadAction {
            //删除所有存在的通知
            val notifications = getUserData(importNotificationKey) ?: mutableListOf()
            putUserData(importNotificationKey, notifications)
            notifications.forEach { it.expire() }
            notifications.clear()
            //创建通知
            val imported = this.getProperty(importedRepositoriesPropertyKey) ?: listOf()
            val ignored = this.getProperty(ignoredRepositoriesPropertyKey) ?: listOf()
            //获取可能存在的sqlex repository
            allMaybeSqlExSourceRoot
                .filter { it.projectRootRelativePath != null }
                .filter { !imported.contains(it.projectRootRelativePath) && !ignored.contains(it.projectRootRelativePath) }
                .forEach { sourceRoot ->
                    val notification =
                        this.createNotification("发现SqlEx源码目录(${sourceRoot.projectRootRelativePath}),是否导入?")
                    notification.addAction(object : AnAction("导入") {
                        override fun actionPerformed(event: AnActionEvent) {
                            notification.expire()
                            try {
                                this@showMaybeSqlExImportNotification.importRepository(sourceRoot)
                            } catch (e: Exception) {
                                e.message?.let { event.project?.showNotification(it, NotificationType.WARNING) }
                            }
                        }
                    })
                    notification.addAction(object : AnAction("忽略") {
                        override fun actionPerformed(e: AnActionEvent) {
                            notification.expire()
                            this@showMaybeSqlExImportNotification.ignoreRepository(sourceRoot)
                        }
                    })
                    notification.notify(this)
                    notifications.add(notification)
                }
        }
    }
}

//同步Repository
fun Project.syncRepositoryService() {
    repositoryServiceSyncLocker.withLock {
        this.modules.forEach { module ->
            //去掉所有生成的源码标记
            module.unmarkAllGeneratedSourceRoot()
            //删除所有的repository
            module.sqlexRepositoryServices.forEach { service ->
                service.close()
                messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC).removed(service)
            }
            module.sqlexRepositoryServices.clear()
        }
        //删除所有的SqlExDataSource
        this.allSqlExDataSources.forEach { this.removeDataSource(it) }
        //本地文件系统
        val localFileSystem = LocalFileSystem.getInstance()
        //project路径
        val projectPath = this.basePath ?: throw Exception("无法获取项目的根目录")
        //获取导入的repository配置
        val sourceRoot = this.getProperty(importedRepositoriesPropertyKey)
            ?.mapNotNull {
                localFileSystem.refreshAndFindFileByPath(
                    Paths.get(projectPath, it).toAbsolutePath().toString()
                )
            } ?: return
        //新建service
        sourceRoot.forEach {
            val module = it.module ?: return@forEach
            val repositoryServices = module.sqlexRepositoryServices
            val newRepositoryService = SqlExRepositoryService(it)
            repositoryServices.add(newRepositoryService)
            messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC)
                .created(newRepositoryService)
            newRepositoryService.refresh()
        }
    }
}

//导入Repository
fun Project.importRepository(sourceRoot: VirtualFile) {
    repositoryServiceSyncLocker.withLock {
        if (sourceRoot.findChild(SqlExConfigFileName)?.isSqlExConfig == false)
            return
        //判断repository是否已经存在
        if (this.sqlexRepositoryServices.any { it.sourceRoot == sourceRoot })
            throw Exception("该Repository已被导入")
        //获取source root对应的module
        val module = sourceRoot.module ?: return
        val repositoryServices = module.sqlexRepositoryServices
        val newRepositoryService = SqlExRepositoryService(sourceRoot)
        repositoryServices.add(newRepositoryService)
        messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC).created(newRepositoryService)
        newRepositoryService.refresh()
        //保存导入配置
        this.setProperty(
            importedRepositoriesPropertyKey,
            this.sqlexRepositoryServices.mapNotNull { it.sourceRoot.projectRootRelativePath })
    }
}

//删除导入的Repository
fun Project.removeImportedRepository(sourceRoot: VirtualFile) {
    repositoryServiceSyncLocker.withLock {
        val repositoryServices = sourceRoot.module?.sqlexRepositoryServices ?: return
        val repositoryService = repositoryServices.find { it.sourceRoot == sourceRoot } ?: return
        repositoryService.close()
        repositoryServices.remove(repositoryService)
        messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC).removed(repositoryService)
        //将其存入忽略列表
        ignoreRepository(sourceRoot)
        //保存导入配置
        this.setProperty(
            importedRepositoriesPropertyKey,
            this.sqlexRepositoryServices.mapNotNull { it.sourceRoot.projectRootRelativePath })
    }
}

fun Project.ignoreRepository(sourceRoot: VirtualFile) {
    val ignored = this.getProperty(ignoredRepositoriesPropertyKey)?.toMutableList() ?: mutableListOf()
    ignored.add(sourceRoot.projectRootRelativePath ?: return)
    this.setProperty(ignoredRepositoriesPropertyKey, ignored.distinct())
}


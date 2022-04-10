package me.danwi.sqlex.idea.service

import com.intellij.notification.NotificationType
import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.progress.ProcessCanceledException
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VfsUtilCore
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import me.danwi.sqlex.idea.listener.SqlExRepositoryEventListener
import me.danwi.sqlex.idea.util.extension.*
import me.danwi.sqlex.parser.RepositoryBuilder
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryMethodException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.util.schemaFileVersion
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock

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

    //SqlEx过期是否自动刷新
    var autoRefresh = true
        set(value) {
            val oldValue = field
            field = value
            if (oldValue != value)
                messagePublisher.autoRefreshChanged(this, value)
            if (!isValid && !oldValue && value)
                refresh()
        }

    //SqlEx代码仓库
    var repository: SqlExRepository? = null
        private set
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

    //endregion

    init {
        //设置数据库方言
        invokeAndWaitIfNeeded {
            SqlDialectMappings.getInstance(project)
                .setMapping(sourceRoot, MysqlDialect.INSTANCE)
        }
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
                            //暴露任务指示器
                            this@SqlExRepositoryService.indicator = indicator
                            //需要清理的builder
                            var builderToClean: RepositoryBuilder? = null
                            //需要清理的repository
                            var repositoryToClean: SqlExRepository? = null
                            try {
                                //开始刷新
                                messagePublisher.beforeRefresh(this@SqlExRepositoryService)
                                indicator.isIndeterminate = true
                                indicator.text = "SqlEx: 等待任务开始"
                                indicator.checkCanceled()

                                //解析配置
                                indicator.isIndeterminate = true
                                indicator.text = "SqlEx: 解析配置"
                                val configFileContent =
                                    sourceRoot.configFile?.textContent ?: throw Exception("无法读取配置文件")
                                val config = createSqlExConfig(configFileContent)

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
                                val ddlScript = parserRepository.DDL
                                val repository = SqlExRepository(project, parserRepository)
                                repositoryToClean = repository
                                this@SqlExRepositoryService.repository = repository

                                indicator.checkCanceled()

                                //同步DatabaseTools
                                indicator.text = "SqlEx: 同步 Database Tools"

                                val rootPackage = config.rootPackage ?: throw Exception("无法获取SqlEx Config的根包名")

                                //同步数据源
                                //在现存的数据源中查找
                                var dataSource = project.findDataSource(sourceRoot)
                                //如果现有的数据源没有,则新建一个
                                if (dataSource == null)
                                    dataSource = project.addDataSource(rootPackage, sourceRoot)
                                //设置关键信息
                                dataSource.sqlexName = rootPackage
                                dataSource.ddl = ddlScript

                                indicator.checkCanceled()

                                //解析method文件
                                indicator.text = "SqlEx: 解析Method"
                                indicator.isIndeterminate = false
                                methodFiles.forEachIndexed { index, file ->
                                    indicator.text =
                                        "SqlEx: 解析Method(${index + 1}/${methodFiles.size}) ${file.sourceRootRelativePath}"
                                    indicator.fraction = (index + 1) / methodFiles.size.toDouble() / 2.0 + 0.5
                                    repository.updateMethodFile(file)
                                    indicator.checkCanceled()
                                }

                                isValid = true
                            } catch (e: Exception) {
                                //清理builder
                                builderToClean?.close()
                                //清理repository
                                repositoryToClean?.close()
                                when (e) {
                                    is ProcessCanceledException -> project.showNotification(
                                        "SqlEx索引更新被取消",
                                        NotificationType.WARNING
                                    )
                                    is SqlExRepositorySchemaException -> project.showNotification(
                                        "解析Schema文件 [${e.relativePath}] 错误: ${e.message}",
                                        NotificationType.ERROR
                                    )
                                    is SqlExRepositoryMethodException -> project.showNotification(
                                        "解析Method文件 [${e.relativePath}] 错误: ${e.message}",
                                        NotificationType.ERROR
                                    )
                                    else -> project.showNotification(
                                        "重建SqlEx索引时发生错误: ${e.message ?: "未知错误"}, 索引构建失败",
                                        NotificationType.ERROR
                                    )
                                }
                            } finally {
                                this@SqlExRepositoryService.indicator = null
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

    fun close() {
        stopRefresh()
        val dataSource = project.findDataSource(sourceRoot)
        if (dataSource != null)
            project.removeDataSource(dataSource, sourceRoot)
        this.repository?.close()
        this.repository = null
    }
}

//同步锁
val repositoryServiceSyncLocker = ReentrantLock()

//同步Module上RepositoryService列表
fun syncRepositoryService(project: Project) {
    //事件消息发布器
    val messagePublisher = project.messageBus.syncPublisher(SqlExRepositoryEventListener.REPOSITORY_SERVICE_TOPIC)
    //加锁
    repositoryServiceSyncLocker.withLock {
        //在处理之前先删除所有遗留的垃圾数据源
        val sqlexSourceRoots =
            project.modules
                .flatMap { m -> m.sourceRoots.filter { s -> s.isSqlExSourceRoot } }
                .map { it.path }
        project.allSqlExDataSources
            .filter { !sqlexSourceRoots.contains(it.sqlexSourceRootPath) }
            .forEach { project.removeDataSource(it) }

        //处理所有的module
        project.modules.forEach { module ->
            //拿到这个模块下的所有SqlEx source root
            val sourceRoots = module.sourceRoots.filter { it.isSqlExSourceRoot }
            val sqlExRepositoryServices = module.sqlexRepositoryServices
            //删除已经不存在的repository
            val repositoriesToDelete = sqlExRepositoryServices.filter { r -> sourceRoots.none { it == r.sourceRoot } }
            repositoriesToDelete.forEach {
                it.close()
                sqlExRepositoryServices.remove(it)
                messagePublisher.removed(it)
            }
            //添加新的repository
            sourceRoots
                .filter { s -> sqlExRepositoryServices.none { it.sourceRoot == s } }
                .map { SqlExRepositoryService(it) }
                .forEach {
                    sqlExRepositoryServices.add(it)
                    messagePublisher.created(it)
                    it.refresh()
                }
        }
    }
}


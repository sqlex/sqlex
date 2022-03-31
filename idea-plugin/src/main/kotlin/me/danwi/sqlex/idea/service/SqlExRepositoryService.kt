package me.danwi.sqlex.idea.service

import com.intellij.ide.highlighter.JavaFileType
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.application.invokeAndWaitIfNeeded
import com.intellij.openapi.application.runReadAction
import com.intellij.openapi.progress.ProcessCanceledException
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VfsUtilCore
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiFileFactory
import com.intellij.psi.PsiJavaFile
import com.intellij.sql.database.SqlDataSourceImpl
import com.intellij.sql.dialects.SqlDialectMappings
import com.intellij.sql.dialects.mysql.MysqlDialect
import me.danwi.sqlex.idea.listener.SqlExRepositoryEventListener
import me.danwi.sqlex.idea.util.extension.*
import me.danwi.sqlex.parser.JavaFile
import me.danwi.sqlex.parser.Repository
import me.danwi.sqlex.parser.RepositoryBuilder
import me.danwi.sqlex.parser.config.createSqlExConfig
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock

//生成的虚拟java文件前缀
const val SQLEX_GENERATED_PREFIX = "SQLEX_GENERATE_FROM:"

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

    //所有的Java类
    val allJavaClass: Array<PsiClass>
        get() {
            val classes = javaClassCache.values
            val innerClass = classes.flatMap { it.innerClasses.toList() }
            return (classes + innerClass).toTypedArray()
        }

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

    //解析库
    private var repository: Repository? = null

    //数据源
    private var dataSource: SqlDataSourceImpl? = null

    //生成的javaClass缓存
    private val javaClassCache = mutableMapOf<String, PsiClass>()

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
                            var repositoryToClean: Repository? = null
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
                                    .map {
                                        Pair(
                                            Regex("^(\\d+)").find(it.name)?.groups?.get(0)?.value?.toInt(),
                                            it
                                        )
                                    }
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
                                    indicator.text =
                                        "SqlEx: 解析Schema(${index + 1}/${sortedSchemaFiles.size}) ${file.sourceRootRelativePath}"
                                    indicator.fraction = (index + 1) / sortedSchemaFiles.size.toDouble() / 2.0
                                    builder.addSchema(file.textContent ?: throw Exception("无法读取${file.name}的文件内容"))
                                    indicator.checkCanceled()
                                }
                                indicator.isIndeterminate = true
                                val repository = builder.build()
                                repositoryToClean = repository
                                this@SqlExRepositoryService.repository = repository

                                indicator.checkCanceled()

                                //同步DatabaseTools
                                indicator.text = "SqlEx: 同步 Database Tools"

                                val rootPackage = config.rootPackage ?: throw Exception("无法获取SqlEx Config的根包名")

                                //同步数据源
                                var dataSource = this@SqlExRepositoryService.dataSource
                                //如果临时变量数据源不存在,则在现存的数据源中查找
                                if (dataSource == null)
                                    dataSource = project.findDataSource(rootPackage)
                                //如果现有的数据源没有,则新建一个
                                if (dataSource == null)
                                    dataSource = project.addDataSource(rootPackage, repository.DDL, sourceRoot)
                                //设置关键信息
                                dataSource.sqlexName = rootPackage
                                dataSource.ddl = repository.DDL
                                //保存到临时变量
                                this@SqlExRepositoryService.dataSource = dataSource

                                indicator.checkCanceled()

                                //解析method文件
                                indicator.text = "SqlEx: 解析Method"
                                indicator.isIndeterminate = false
                                methodFiles.forEachIndexed { index, file ->
                                    indicator.text =
                                        "SqlEx: 解析Method(${index + 1}/${methodFiles.size}) ${file.sourceRootRelativePath}"
                                    indicator.fraction = (index + 1) / methodFiles.size.toDouble() / 2.0 + 0.5
                                    updateSqlMethodFile(file)
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
                                    else -> project.showNotification(e.message ?: "未知错误", NotificationType.ERROR)
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

    fun generateJavaFile(file: VirtualFile?): JavaFile? {
        if (file == null)
            return null
        return repository?.generateJavaFile(
            file.sourceRootRelativePath ?: throw Exception("无法获取文件${file.name}的相对路径"),
            file.textContent ?: throw Exception("无法读取文件${file.name}的内容")
        )
    }

    fun updateSqlMethodFile(file: VirtualFile?) {
        if (file == null)
            return
        val javaFile = generateJavaFile(file) ?: return
        val javaClass = runReadAction {
            (PsiFileFactory.getInstance(project)
                .createFileFromText(
                    "${SQLEX_GENERATED_PREFIX}${javaFile.javaClass}.java",
                    JavaFileType.INSTANCE,
                    javaFile.source
                ) as PsiJavaFile).classes.firstOrNull()
        } ?: return

        javaClassCache[file.path] = javaClass
    }

    fun removeSqlMethodFile(filePath: String) {
        javaClassCache.remove(filePath)
    }

    fun removeSqlMethodFile(file: VirtualFile?) {
        javaClassCache.remove(file?.path ?: return)
    }

    fun findClasses(qualifiedPackage: String): Array<PsiClass> {
        return javaClassCache.values
            .filter { it.qualifiedPackageName == qualifiedPackage }
            .toTypedArray()
    }

    fun findClass(qualifiedName: String): PsiClass? {
        return allJavaClass.firstOrNull { it.qualifiedName == qualifiedName }
    }

    fun findClassByName(name: String): PsiClass? {
        return allJavaClass.firstOrNull { it.name == name }
    }

    fun isBelong(sourceRoot: VirtualFile): Boolean {
        return this.sourceRoot == sourceRoot
    }

    fun close() {
        stopRefresh()
        val dataSource = this.dataSource
        if (dataSource != null)
            project.removeDataSource(dataSource, sourceRoot)
        this.repository?.close()
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
        //处理所有的module
        project.modules.forEach { module ->
            //拿到这个模块下的所有SqlEx source root
            val sourceRoots = module.sourceRoots.filter { it.isSqlExSourceRoot }
            val sqlExRepositoryServices = module.sqlexRepositoryServices
            //删除已经不存在的repository
            val repositoriesToDelete = sqlExRepositoryServices.filter { r -> sourceRoots.none { r.isBelong(it) } }
            repositoriesToDelete.forEach {
                it.close()
                sqlExRepositoryServices.remove(it)
                messagePublisher.removed(it)
            }
            //添加新的repository
            sourceRoots
                .filter { s -> sqlExRepositoryServices.none { it.isBelong(s) } }
                .map { SqlExRepositoryService(it) }
                .forEach {
                    sqlExRepositoryServices.add(it)
                    messagePublisher.created(it)
                    it.refresh()
                }
        }
    }
}


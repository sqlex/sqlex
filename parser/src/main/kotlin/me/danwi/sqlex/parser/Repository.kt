package me.danwi.sqlex.parser

import me.danwi.sqlex.common.SQLUtils
import me.danwi.sqlex.parser.config.SqlExConfig
import me.danwi.sqlex.parser.config.parseSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryException
import me.danwi.sqlex.parser.exception.SqlExRepositoryGenerateException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.generate.GeneratedEntityFile
import me.danwi.sqlex.parser.generate.GeneratedMethodFile
import me.danwi.sqlex.parser.generate.GeneratedRepositoryFile
import me.danwi.sqlex.parser.generate.GeneratedTableFile
import me.danwi.sqlex.parser.util.*
import java.io.File
import java.nio.file.Paths
import java.util.concurrent.atomic.AtomicInteger

//数据名称索引
private val index = AtomicInteger()

class RepositoryBuilder(private val config: SqlExConfig) {
    private val databaseName = "sqlex_${index.getAndIncrement()}"

    private val foreignDatabaseNameMapping = mutableMapOf<String, String>()

    private val foreignSchemas = mutableMapOf<String, String>()

    private val session: Session = Session(databaseName)

    private val rootPackageRelativePath =
        config.rootPackage.packageNameToRelativePath

    private var currentSchemaVersion = -1

    private val schemaContentCache = mutableListOf<String>()

    fun addForeignSchema(databaseName: String, ddl: String) {
        val mappingDatabaseName = "sqlex_${index.getAndIncrement()}"
        val foreignSession = Session(mappingDatabaseName)
        try {
            foreignSession.executeScript(ddl)
            foreignDatabaseNameMapping[databaseName] = mappingDatabaseName
            foreignSchemas[databaseName] = ddl
        } catch (e: Exception) {
            foreignSession.dropDatabaseAndClose()
            throw SqlExRepositorySchemaException("foreign database $databaseName", e.message)
        } finally {
            foreignSession.close()
        }
    }

    fun addSchema(relativePath: String, script: String) {
        try {
            //schema文件必须包含在根包下面
            if (!relativePath.startsWith(rootPackageRelativePath)) {
                throw SqlExRepositorySchemaException(
                    relativePath,
                    "Schema文件必须在根包下,当前根包: ${rootPackageRelativePath.relativePathToPackageName} ,Schema文件所在包: ${relativePath.relativePathToPackageName}"
                )
            }
            //获取schema的版本
            val version = relativePath.schemaFileVersion
            if (version != currentSchemaVersion + 1)
                if (currentSchemaVersion == -1)
                    throw SqlExRepositorySchemaException(relativePath, "Schema文件的版本必须从0开始")
                else
                    throw SqlExRepositorySchemaException(
                        relativePath,
                        "Schema文件版本必须连续且不重复,当前需要 ${currentSchemaVersion + 1} 版本,实际提供 ${version} 版本"
                    )
            //执行
            session.executeScript(script)
            //添加到缓存
            schemaContentCache.add(script)
            currentSchemaVersion++
        } catch (e: Exception) {
            try {
                session.execute("drop database $databaseName")
            } finally {
                session.close()
                throw SqlExRepositorySchemaException(relativePath, e.message)
            }
        }
    }

    fun build(): Repository {
        try {
            return Repository(
                Session(databaseName),
                config,
                schemaContentCache,
                foreignDatabaseNameMapping,
                foreignSchemas
            )
        } catch (e: Exception) {
            session.dropDatabaseAndClose()
            throw e
        } finally {
            session.close()
        }
    }

    fun close() {
        //删除外部database
        foreignDatabaseNameMapping.values.forEach {
            try {
                session.execute("drop database $it")
            } catch (_: Exception) {
            }
        }
        session.dropDatabaseAndClose()
    }
}

class Repository(
    private val session: Session,
    private val config: SqlExConfig,
    private val schemas: List<String>,
    private val foreignDatabaseNameMapping: Map<String, String>,
    private val foreignSchemas: Map<String, String>
) {
    private val rootPackage = config.rootPackage
    private val converters = config.converters

    //生成SqlEx Repository顶级类
    fun generateRepositoryClassFile(
        tableClassNames: List<String>,
        methodClassNames: List<String>
    ): GeneratedRepositoryFile {
        return GeneratedRepositoryFile(
            rootPackage,
            converters,
            schemas,
            tableClassNames,
            methodClassNames,
            this
        )
    }

    //生成所有表的实体类/表操作类
    fun generateEntityAndTableClassFiles(): List<Pair<GeneratedEntityFile, GeneratedTableFile>> {
        return session.allTables.map {
            val entityFile = GeneratedEntityFile(rootPackage, it, this)
            val tableFile = GeneratedTableFile(rootPackage, it, entityFile.qualifiedName, this)
            Pair(entityFile, tableFile)
        }
    }

    //生成SqlEx Method类
    fun generateMethodClassFile(relativePath: String, content: String): GeneratedMethodFile {
        return GeneratedMethodFile(rootPackage, relativePath, content, this)
    }

    fun close() {
        //删除外部database
        foreignDatabaseNameMapping.values.forEach {
            try {
                session.execute("drop database $it")
            } catch (_: Exception) {
            }
        }
        session.dropDatabaseAndClose()
    }

    /******** session 包装 ********/
    //获取自己数据库的ddl
    val selfDDL: String = session.DDL

    //包含外部数据库
    val DDL: String
        get() {
            val builder = StringBuilder()
            builder.append(selfDDL)
            builder.append('\n')
            //添加外部ddl
            foreignDatabaseNameMapping.forEach { (databaseName, _) ->
                val ddl = foreignSchemas[databaseName] ?: return@forEach
                builder.append("\n# schema of $databaseName\n\n")
                builder.append("create database $databaseName;\n\n")
                builder.append("use $databaseName;\n\n")
                builder.append(ddl)
                builder.append('\n')
            }
            return builder.toString()
        }

    fun getTableInfo(tableName: String): TableInfo {
        return session.getTableInfo(tableName)
    }

    fun getStatementInfo(sql: String): StatementInfo {
        return session.getStatementInfo(sql)
    }

    fun getPlanInfo(sql: String): PlanInfo {
        return session.getPlanInfo(SQLUtils.replaceDatabaseName(sql, foreignDatabaseNameMapping))
    }

    fun getSQLsOfScript(script: String): Array<String> {
        return session.getSQLsOfScript(script)
    }
}

//给定一个sqlex的source root,将代码生成到指定到文件中
fun generateRepositorySource(sourceRoot: File, javaOutputDir: File, resourceOutputDir: File) {
    //获取目录下的所有文件
    val files = sourceRoot.walk()
    //读取配置文件
    var tempConfigContent: String? = null
    files.maxDepth(1)
        .filter { it.isFile && it.absolutePath.isSqlExConfigFilePath }
        .forEach {
            if (tempConfigContent == null)
                tempConfigContent = it.readText()
            else
                throw SqlExRepositoryException("${sourceRoot.absolutePath}下有多个sqlex配置文件")
        }
    val config = parseSqlExConfig(tempConfigContent ?: return)
    //计算缓存 TODO: 目前不支持sqlex文件夹外的外部脚本的变更发现
    val sourceRootCacheHash = sourceRoot.computeCacheHash
    val javaOutputDirIsUpdate =
        javaOutputDir.sourceCacheHash == sourceRootCacheHash
                && javaOutputDir.selfCacheHash == javaOutputDir.computeCacheHash
    val resourceOutputDirIsUpdate =
        resourceOutputDir.sourceCacheHash == sourceRootCacheHash
                && resourceOutputDir.selfCacheHash == resourceOutputDir.computeCacheHash
    //如果文件均未变更
    if (javaOutputDirIsUpdate && resourceOutputDirIsUpdate) {
        return
    }
    //输出目录清理
    javaOutputDir.deleteRecursively()
    javaOutputDir.mkdirs()
    resourceOutputDir.deleteRecursively()
    resourceOutputDir.mkdirs()
    //开始构建,读取外部schema
    val foreignSchemaFiles = config.foreign.map { (name, file) ->
        val schemaFile = Paths.get(sourceRoot.absolutePath, file).toFile()
        try {
            return@map Pair(name, schemaFile.readText())
        } catch (e: Exception) {
            throw SqlExRepositoryGenerateException(schemaFile.absolutePath, "无法读取外部Schema文件")
        }
    }
    //构建repository
    val builder = RepositoryBuilder(config)
    foreignSchemaFiles.forEach { (databaseName, ddl) -> builder.addForeignSchema(databaseName, ddl) }
    //获取到schema文件集合
    val schemaFiles = files
        .filter { it.isFile && it.name.isSqlExSchemaFilePath }
        .map { Pair(it.name.schemaFileVersion, it) }
        .filter { it.first != null }
        .sortedBy { it.first }
        .map { it.second }
    schemaFiles.forEach { builder.addSchema(it.absolutePath.relativePathTo(sourceRoot.absolutePath), it.readText()) }
    val repository = builder.build()
    try {
        //在生成的源码目录下写入一个文件做标识
        val tagFile = File(javaOutputDir.absolutePath, SqlExGeneratedTagFileName)
        if (!tagFile.exists()) {
            tagFile.parentFile.mkdirs()
            tagFile.writeText("this directory is auto generate by sqlex, DO NOT change anything in it")
        }
        //写入ddl资源
        val ddlContent = repository.selfDDL
        val ddlFile =
            Paths.get(resourceOutputDir.absolutePath, config.rootPackage.packageNameToRelativePath, "ddl.sql").toFile()
        ddlFile.parentFile.mkdirs()
        if (ddlFile.exists())
            throw SqlExRepositoryException("DDL存根重复生成")
        ddlFile.writeText(ddlContent)
        //生成所有的表实体类和表操作类
        val entityAndTableJavaFiles = repository.generateEntityAndTableClassFiles()
        val entityJavaFiles = entityAndTableJavaFiles.map { it.first }
        //写入生成的实体类文件
        entityJavaFiles.forEach {
            val sourceFile = Paths.get(javaOutputDir.absolutePath, it.relativePath).toFile()
            sourceFile.parentFile.mkdirs()
            if (sourceFile.exists())
                throw SqlExRepositoryGenerateException(sourceFile.absolutePath, "重复源码生成")
            sourceFile.writeText(it.source)
        }
        //生成所有的表操作类
        val tableJavaFiles = entityAndTableJavaFiles.map { it.second }
        //写入生成的操作类文件
        tableJavaFiles.forEach {
            val sourceFile = Paths.get(javaOutputDir.absolutePath, it.relativePath).toFile()
            sourceFile.parentFile.mkdirs()
            if (sourceFile.exists())
                throw SqlExRepositoryGenerateException(sourceFile.absolutePath, "重复源码生成")
            sourceFile.writeText(it.source)
        }
        //获取到所有的sqlm文件
        val methodJavaFiles = files
            .filter { it.isFile && it.absolutePath.isSqlExMethodFilePath }
            .map {
                repository.generateMethodClassFile(
                    it.absolutePath.relativePathTo(sourceRoot.absolutePath),
                    it.readText()
                )
            }
        //生成顶级Repository类文件
        val repositoryJavaFile =
            repository.generateRepositoryClassFile(
                tableJavaFiles.map { it.qualifiedName },
                methodJavaFiles.map { it.qualifiedName }.toList()
            )
        val repositorySourceFile = Paths.get(javaOutputDir.absolutePath, repositoryJavaFile.relativePath).toFile()
        repositorySourceFile.parentFile.mkdirs()
        if (repositorySourceFile.exists())
            throw SqlExRepositoryGenerateException(repositorySourceFile.absolutePath, "重复源码生成")
        repositorySourceFile.writeText(repositoryJavaFile.source)
        //写入生成的SqlEx Method类文件
        methodJavaFiles.forEach {
            val sourceFile = Paths.get(javaOutputDir.absolutePath, it.relativePath).toFile()
            sourceFile.parentFile.mkdirs()
            if (sourceFile.exists())
                throw SqlExRepositoryGenerateException(sourceFile.absolutePath, "重复源码生成")
            sourceFile.writeText(it.source)
        }
        //写入缓存
        javaOutputDir.sourceCacheHash = sourceRootCacheHash
        javaOutputDir.selfCacheHash = javaOutputDir.computeCacheHash
        resourceOutputDir.sourceCacheHash = sourceRootCacheHash
        resourceOutputDir.selfCacheHash = resourceOutputDir.computeCacheHash
    } finally {
        repository.close()
    }
}

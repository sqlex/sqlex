package me.danwi.sqlex.parser

import me.danwi.sqlex.parser.config.SqlExConfig
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryException
import me.danwi.sqlex.parser.exception.SqlExRepositoryGenerateException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.generate.GeneratedJavaFile
import me.danwi.sqlex.parser.generate.GeneratedMethodFile
import me.danwi.sqlex.parser.generate.GeneratedRepositoryFile
import me.danwi.sqlex.parser.util.*
import java.io.File
import java.nio.file.Paths
import java.util.concurrent.atomic.AtomicInteger

//数据名称索引
private val index = AtomicInteger()

class RepositoryBuilder(private val config: SqlExConfig) {
    private val databaseName = "sqlex_${index.getAndIncrement()}"

    private val session: Session = Session(databaseName)

    private val rootPackageRelativePath =
        config.rootPackage?.packageNameToRelativePath ?: throw SqlExRepositoryException("无法获取根包信息")

    private var currentSchemaVersion = -1

    private val schemaContentCache = mutableListOf<String>()

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
            return Repository(databaseName, Session(databaseName), config, schemaContentCache)
        } catch (e: Exception) {
            try {
                session.execute("drop database $databaseName")
            } finally {
                throw e
            }
        } finally {
            session.close()
        }
    }

    fun close() {
        try {
            session.execute("drop database $databaseName")
        } catch (_: Exception) {
        } finally {
            session.close()
        }
    }
}

class Repository(
    private val databaseName: String,
    private val session: Session,
    private val config: SqlExConfig,
    private val schemas: List<String>
) {
    private val rootPackage = config.rootPackage ?: throw SqlExRepositoryException("无法获取根包信息")
    private val converters = config.converters

    //获取session
    fun getSession(): Session {
        return session
    }

    //SqlEx Repository顶级类
    val repositoryJavaFile: GeneratedJavaFile by lazy {
        GeneratedRepositoryFile(
            rootPackage,
            converters,
            schemas,
            session
        )
    }

    //生成整个java文件
    fun generateJavaFile(relativePath: String, content: String): GeneratedMethodFile {
        return GeneratedMethodFile(rootPackage, relativePath, content, session)
    }

    fun close() {
        try {
            session.execute("drop database $databaseName")
        } finally {
            session.close()
        }
    }
}

//给定一个sqlex的source root,将代码生成到指定到文件中
fun generateRepositorySource(sourceRoot: File, outputDir: File) {
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
    val config = createSqlExConfig(tempConfigContent ?: return)
    //获取到schema文件集合
    val schemaFiles = files
        .filter { it.isFile && it.name.isSqlExSchemaFilePath }
        .map { Pair(it.name.schemaFileVersion, it) }
        .filter { it.first != null }
        .sortedBy { it.first }
        .map { it.second }
    //构建repository
    val builder = RepositoryBuilder(config)
    schemaFiles.forEach { builder.addSchema(it.absolutePath.relativePathTo(sourceRoot.absolutePath), it.readText()) }
    val repository = builder.build()
    try {
        //在生成的源码目录下写入一个文件做标识
        val tagFile = File(outputDir.absolutePath, SqlExGeneratedTagFileName)
        if (!tagFile.exists()) {
            tagFile.parentFile.mkdirs()
            tagFile.writeText("this directory is auto generate by sqlex, DO NOT change anything in it")
        }
        //生成顶级Repository类文件
        val repositoryJavaFile = repository.repositoryJavaFile
        val repositorySourceFile = Paths.get(outputDir.absolutePath, repositoryJavaFile.relativePath).toFile()
        repositorySourceFile.parentFile.mkdirs()
        if (repositorySourceFile.exists())
            throw SqlExRepositoryGenerateException(repositorySourceFile.absolutePath, "重复源码生成")
        repositorySourceFile.writeText(repositoryJavaFile.source)
        //获取到所有的sqlm文件
        files
            .filter { it.isFile && it.absolutePath.isSqlExMethodFilePath }
            .map {
                repository.generateJavaFile(
                    it.absolutePath.relativePathTo(sourceRoot.absolutePath),
                    it.readText()
                )
            }
            .forEach {
                val sourceFile = Paths.get(outputDir.absolutePath, it.relativePath).toFile()
                sourceFile.parentFile.mkdirs()
                if (sourceFile.exists())
                    throw SqlExRepositoryGenerateException(sourceFile.absolutePath, "重复源码生成")
                sourceFile.writeText(it.source)
            }
    } finally {
        repository.close()
    }
}

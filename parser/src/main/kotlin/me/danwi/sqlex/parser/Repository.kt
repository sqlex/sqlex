package me.danwi.sqlex.parser

import com.squareup.javapoet.AnnotationSpec
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.annotation.source.SqlExConfigFileSource
import me.danwi.sqlex.core.annotation.source.SqlExMethodFileSource
import me.danwi.sqlex.core.annotation.source.SqlExRepositorySource
import me.danwi.sqlex.core.annotation.source.SqlExSchemaFileSource
import me.danwi.sqlex.parser.config.SqlExConfig
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.exception.SqlExRepositoryException
import me.danwi.sqlex.parser.exception.SqlExRepositorySchemaException
import me.danwi.sqlex.parser.generate.GeneratedEntityFile
import me.danwi.sqlex.parser.generate.GeneratedMethodFile
import me.danwi.sqlex.parser.generate.GeneratedRepositoryFile
import me.danwi.sqlex.parser.generate.GeneratedTableFile
import me.danwi.sqlex.parser.util.*
import java.io.File
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
    val session: Session,
    private val config: SqlExConfig,
    private val schemas: List<String>
) {
    private val rootPackage = config.rootPackage ?: throw SqlExRepositoryException("无法获取根包信息")
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
            session
        )
    }

    //生成所有表的实体类/表操作类
    fun generateEntityAndTableClassFiles(): List<Pair<GeneratedEntityFile, GeneratedTableFile>> {
        return session.allTables.map {
            val entityFile = GeneratedEntityFile(rootPackage, it, session)
            val tableFile = GeneratedTableFile(rootPackage, it, entityFile.qualifiedName, session)
            Pair(entityFile, tableFile)
        }
    }

    //生成SqlEx Method类
    fun generateMethodClassFile(relativePath: String, content: String): GeneratedMethodFile {
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

//给定一个source root,将sqlex代码代码生成一个存根类
fun generateRepositorySource(sourceRoot: File, outputDir: File) {
    //获取目录下定所有文件
    val files = sourceRoot.walk()
    //读取配置文件
    var configContent: String? = null
    files.maxDepth(1)
        .filter { it.isFile && it.absolutePath.isSqlExConfigFilePath }
        .forEach {
            if (configContent == null)
                configContent = it.readText()
            else
                throw SqlExRepositoryException("${sourceRoot.absolutePath}下有多个sqlex配置文件")
        }
    //解析配置
    val config = createSqlExConfig(configContent ?: return)
    //源码存根类
    val sourceStub = TypeSpec.classBuilder("RepositorySourceStub")
        .addAnnotation(SqlExRepositorySource::class.java)
        .addAnnotation(
            AnnotationSpec.builder(SqlExConfigFileSource::class.java).addMember("value", configContent).build()
        )
    val builder = RepositoryBuilder(config)
    //获取到所有的sqls文件
    val schemaFiles = files
        .filter { it.isFile && it.name.isSqlExSchemaFilePath }
        .map { Pair(it.absolutePath.relativePathTo(sourceRoot.absolutePath), it.readText()) }
    //添加到源码存根中
    schemaFiles.forEach {
        sourceStub.addAnnotation(
            AnnotationSpec.builder(SqlExSchemaFileSource::class.java)
                .addMember("relativePath", it.first)
                .addMember("content", it.second)
                .build()
        )
    }
    //构建repository
    schemaFiles
        .filter { it.first.schemaFileVersion != null }
        .sortedBy { it.first.schemaFileVersion }
        .forEach { builder.addSchema(it.first, it.second) }
    val repository = builder.build()
    //获取到所有的sqlm文件,并添加到源码存根中
    files
        .filter { it.isFile && it.absolutePath.isSqlExMethodFilePath }
        .map { (Pair(it.absolutePath.relativePathTo(sourceRoot.absolutePath), it.readText())) }
        .forEach {
            sourceStub.addAnnotation(
                AnnotationSpec.builder(SqlExMethodFileSource::class.java)
                    .addMember("relativePath", it.first)
                    .addMember("content", it.second)
                    .build()
            )
        }
    try {
        //在生成的源码目录下写入一个文件做标识
        val tagFile = File(outputDir.absolutePath, SqlExGeneratedTagFileName)
        if (!tagFile.exists()) {
            tagFile.parentFile.mkdirs()
            tagFile.writeText("this directory is auto generate by sqlex, DO NOT change anything in it")
        }
        //
    } finally {
        repository.close()
    }
}

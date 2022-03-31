package me.danwi.sqlex.gradle

import me.danwi.sqlex.parser.RepositoryBuilder
import me.danwi.sqlex.parser.config.createSqlExConfig
import me.danwi.sqlex.parser.util.isSqlExConfigFilePath
import me.danwi.sqlex.parser.util.isSqlExMethodFilePath
import me.danwi.sqlex.parser.util.isSqlExSchemaFilePath
import me.danwi.sqlex.parser.util.windowsPathNormalize
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.tasks.*
import java.io.File
import java.nio.file.Paths

abstract class GenerateSqlExTask : DefaultTask() {
    @Internal
    lateinit var srcSet: SourceSet

    @SkipWhenEmpty
    @InputFiles
    @PathSensitive(PathSensitivity.RELATIVE)
    lateinit var sqlexSourceDirs: Set<File>

    @OutputDirectory
    var outputDir: File = Paths.get(temporaryDir.path).toFile()

    init {
        //不缓存,每次build都自己生成
        outputs.upToDateWhen { false }
    }

    @TaskAction
    fun generate() {
        //准备生成目录
        outputDir.deleteRecursively()
        outputDir.mkdirs()

        //把源码生成目录添加java插件的源码目录中去
        srcSet.java.srcDir(outputDir)

        //针对存在的sqlex做生成
        sqlexSourceDirs.forEach { generateRepository(it) }
    }

    private fun generateRepository(sourceRoot: File) {
        //获取目录下的所有文件
        val files = sourceRoot.walk()
        //读取配置文件
        var tempConfigContent: String? = null
        files.maxDepth(1)
            .filter { it.isFile && it.path.isSqlExConfigFilePath }
            .forEach {
                if (tempConfigContent == null)
                    tempConfigContent = it.readText()
                else
                    throw GradleException("${sourceRoot.path}下有多个sqlex配置文件")
            }
        val config = createSqlExConfig(tempConfigContent ?: return)
        //获取到schema文件集合
        val schemaFiles = files
            .filter { it.isFile && it.name.isSqlExSchemaFilePath }
            .map {
                Pair(
                    Regex("^(\\d+)").find(it.name)?.groups?.get(0)?.value?.toInt(),
                    it
                )
            }
            .filter { it.first != null }
            .sortedBy { it.first }
            .map { it.second.readText() }
        //构建repository
        val builder = RepositoryBuilder(config)
        schemaFiles.forEach { builder.addSchema(it) }
        val repository = builder.build()
        try {
            //获取到所有的sqlm文件
            files
                .filter { it.isFile && it.path.isSqlExMethodFilePath }
                .map {
                    Pair(
                        it.path.windowsPathNormalize.removePrefix(sourceRoot.path.windowsPathNormalize)
                            .removePrefix("/"), it.readText()
                    )
                }
                .map { repository.generateJavaFile(it.first, it.second) }
                .forEach {
                    val sourceFile = Paths.get(outputDir.path, it.relativePath).toFile()
                    sourceFile.parentFile.mkdirs()
                    if (sourceFile.exists())
                        throw GradleException("重复源码生成: ${sourceFile.path}")
                    sourceFile.writeText(it.source)
                }
        } finally {
            repository.close()
        }
    }
}
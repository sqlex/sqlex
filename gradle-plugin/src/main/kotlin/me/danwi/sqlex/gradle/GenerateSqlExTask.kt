package me.danwi.sqlex.gradle

import me.danwi.sqlex.parser.generateRepositorySource
import org.gradle.api.DefaultTask
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
        sqlexSourceDirs.forEach { generateRepositorySource(it, outputDir) }
    }
}
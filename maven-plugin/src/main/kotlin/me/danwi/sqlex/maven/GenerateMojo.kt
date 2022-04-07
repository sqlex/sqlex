package me.danwi.sqlex.maven

import me.danwi.sqlex.parser.generateRepositorySource
import me.danwi.sqlex.parser.util.SqlExConfigFileName
import org.apache.maven.plugin.AbstractMojo
import org.apache.maven.plugins.annotations.LifecyclePhase
import org.apache.maven.plugins.annotations.Mojo
import org.apache.maven.plugins.annotations.Parameter
import org.apache.maven.project.MavenProject
import java.io.File

@Mojo(name = "generate", defaultPhase = LifecyclePhase.GENERATE_SOURCES)
class GenerateMojo : AbstractMojo() {
    @Parameter(readonly = true, defaultValue = "\${project}")
    private var project: MavenProject? = null

    /**
     * SqlEx Repository目录
     */
    @Parameter
    private var sources: Array<File> = arrayOf()

    /**
     * 测试SqlEx Repository目录
     */
    @Parameter
    private var testSources: Array<File> = arrayOf()

    /**
     * 生成源码输出目录(通常不用自己配置)
     */
    @Parameter(defaultValue = "\${project.build.directory}/sqlex")
    private var outputPath: File? = null

    /**
     * 生成测试源码输出目录(通常不用自己配置)
     */
    @Parameter(defaultValue = "\${project.build.directory}/sqlex-test")
    private var testOutputPath: File? = null

    private fun generateSourceTo(sourceRoots: Array<File>, outputPath: File) {
        //获取所有的sqlex source roots
        sourceRoots
            .filter { it.exists() && it.isDirectory }
            .filter {
                val configFile = File(it.absolutePath, SqlExConfigFileName)
                configFile.exists() && configFile.isFile
            }
        //删除旧的源码
        outputPath.deleteRecursively()
        //生成源码
        sourceRoots.forEach {
            log.info("generate [${it.path}] to [${outputPath.path ?: "UNKNOWN"}]")
            generateRepositorySource(it, outputPath)
        }
    }

    override fun execute() {
        val outputPath = outputPath
        if (outputPath != null) {
            generateSourceTo(sources, outputPath)
            project?.addCompileSourceRoot(outputPath.absolutePath)
        }

        val testOutputPath = testOutputPath
        if (testOutputPath != null) {
            generateSourceTo(testSources, testOutputPath)
            project?.addTestCompileSourceRoot(testOutputPath.absolutePath)
        }
    }
}
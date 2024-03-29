package me.danwi.sqlex.maven

import me.danwi.sqlex.parser.generateRepositorySource
import me.danwi.sqlex.parser.util.SqlExConfigFileName
import org.apache.maven.model.Resource
import org.apache.maven.plugin.AbstractMojo
import org.apache.maven.plugins.annotations.LifecyclePhase
import org.apache.maven.plugins.annotations.Mojo
import org.apache.maven.plugins.annotations.Parameter
import org.apache.maven.project.MavenProject
import java.io.File
import java.nio.file.Paths

@Mojo(name = "generate", defaultPhase = LifecyclePhase.GENERATE_SOURCES)
class GenerateMojo : AbstractMojo() {
    @Parameter(readonly = true, defaultValue = "\${project}")
    private var project: MavenProject? = null

    /**
     * SqlEx Repository目录(默认为src/main/sqlex)
     */
    @Parameter
    private var sources: Array<File>? = null

    /**
     * 测试SqlEx Repository目录(默认为src/test/sqlex)
     */
    @Parameter
    private var testSources: Array<File>? = null

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

    private fun generateSourceTo(sourceRoots: Array<File>, javaOutputDir: File, resourceOutputDir: File) {
        //获取所有的sqlex source roots
        sourceRoots
            .filter { it.exists() && it.isDirectory }
            .filter {
                val configFile = File(it.absolutePath, SqlExConfigFileName)
                configFile.exists() && configFile.isFile
            }
        //生成源码
        sourceRoots.forEach {
            log.info("generate [${it.path}] to [${javaOutputDir.path ?: "UNKNOWN"}] [${resourceOutputDir.path ?: "UNKNOWN"}]")
            generateRepositorySource(it, javaOutputDir, resourceOutputDir)
        }
    }

    override fun execute() {
        val project = this.project ?: throw Exception("无法获取任务所在项目")

        val sourceRoots =
            this.sources ?: arrayOf(Paths.get(project.basedir.absolutePath, "src", "main", "sqlex").toFile())
        val outputPath = outputPath
        if (outputPath != null) {
            val javaOutputDir = Paths.get(outputPath.absolutePath, "classes").toFile()
            val resourceOutputDir = Paths.get(outputPath.absolutePath, "resources").toFile()
            generateSourceTo(sourceRoots, javaOutputDir, resourceOutputDir)
            project.addCompileSourceRoot(javaOutputDir.absolutePath)
            project.addResource(Resource().apply { directory = resourceOutputDir.absolutePath })
        }

        val testSourceRoots =
            this.sources ?: arrayOf(Paths.get(project.basedir.absolutePath, "src", "test", "sqlex").toFile())
        val testOutputPath = testOutputPath
        if (testOutputPath != null) {
            val javaOutputDir = Paths.get(testOutputPath.absolutePath, "classes").toFile()
            val resourceOutputDir = Paths.get(testOutputPath.absolutePath, "resources").toFile()
            generateSourceTo(testSourceRoots, javaOutputDir, resourceOutputDir)
            project.addTestCompileSourceRoot(javaOutputDir.absolutePath)
            project.addTestResource(Resource().apply { directory = resourceOutputDir.absolutePath })
        }
    }
}
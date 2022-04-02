package me.danwi.sqlex.maven

import me.danwi.sqlex.parser.generateRepositorySource
import me.danwi.sqlex.parser.util.SqlExConfigFileName
import org.apache.maven.plugin.AbstractMojo
import org.apache.maven.plugins.annotations.LifecyclePhase
import org.apache.maven.plugins.annotations.Mojo
import org.apache.maven.plugins.annotations.Parameter
import org.apache.maven.project.MavenProject
import java.io.File

@Mojo(name = "generate", defaultPhase = LifecyclePhase.PROCESS_SOURCES)
class GenerateMojo : AbstractMojo() {
    @Parameter(readonly = true, defaultValue = "\${project}")
    private var project: MavenProject? = null

    @Parameter(defaultValue = "\${project.build.directory}/sqlex")
    private var outputPath: File? = null

    override fun execute() {
        val outputPath = this.outputPath ?: throw Exception("输出目录未指定")

        //获取所有的sqlex source roots
        val sourceRoots = project?.compileSourceRoots
            ?.map { File(it) }
            ?.filter { it.exists() && it.isDirectory }
            ?.filter {
                val configFile = File(it.absolutePath, SqlExConfigFileName)
                configFile.exists() && configFile.isFile
            } ?: listOf()
        //删除旧的源码
        outputPath.deleteRecursively()
        //生成源码
        sourceRoots.forEach {
            log.info("generate [${it.path}] to [${outputPath.path ?: "UNKNOWN"}]")
            generateRepositorySource(it, outputPath)
        }

        //添加生成的源码目录到源码集合
        project?.addCompileSourceRoot(this.outputPath?.absolutePath ?: return)
    }
}
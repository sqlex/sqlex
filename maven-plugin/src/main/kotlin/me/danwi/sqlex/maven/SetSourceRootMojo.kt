package me.danwi.sqlex.maven

import org.apache.maven.plugin.AbstractMojo
import org.apache.maven.plugins.annotations.LifecyclePhase
import org.apache.maven.plugins.annotations.Mojo
import org.apache.maven.plugins.annotations.Parameter
import org.apache.maven.project.MavenProject
import java.io.File

@Mojo(name = "add-source", defaultPhase = LifecyclePhase.GENERATE_SOURCES)
class SetSourceRootMojo : AbstractMojo() {
    @Parameter(readonly = true, defaultValue = "\${project}")
    private var project: MavenProject? = null

    @Parameter(required = true)
    private var sources: Array<File> = arrayOf()

    override fun execute() {
        sources.forEach {
            project?.addCompileSourceRoot(it.absolutePath)
        }
    }
}
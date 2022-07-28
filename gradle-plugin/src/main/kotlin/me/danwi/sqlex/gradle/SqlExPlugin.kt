package me.danwi.sqlex.gradle

import me.danwi.sqlex.parser.util.pascalName
import org.gradle.api.GradleException
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.plugins.JavaPluginExtension
import org.gradle.api.tasks.SourceSet
import org.gradle.api.tasks.compile.JavaCompile

class SqlExPlugin : Plugin<Project> {
    lateinit var project: Project

    override fun apply(target: Project) {
        this.project = target

        //插件是否成功应用
        var isApplied = false

        //只有配置了java插件,SqlEx插件才会起作用
        project.pluginManager.withPlugin("java") {
            project.extensions.configure(JavaPluginExtension::class.java) {
                //获取Java插件的源码集
                it.sourceSets.forEach(::configureSourceSet)
                isApplied = true
            }
        }

        //当kotlin的kapt开启的时候,保证注解处理器的运行
        project.pluginManager.withPlugin("org.jetbrains.kotlin.kapt") {
            val kapt = project.extensions.findByName("kapt") ?: return@withPlugin
            val kaptClass = kapt::class.java
            val setterMethod =
                kaptClass.methods.find { it.name == "setKeepJavacAnnotationProcessors" } ?: return@withPlugin
            setterMethod.invoke(kapt, true)
        }

        project.afterEvaluate {
            if (!isApplied)
                throw GradleException("SqlEx插件依赖Java插件做代码编译,请在项目中引入Java插件")
            //依赖管理
            configureDependencies()
            //注解处理器管理
            configureAnnotationProcessor()
        }
    }

    //针对java的源码集生成对应的sqlex源码目录,并添加对应的编译任务
    private fun configureSourceSet(sourceSet: SourceSet) {
        //新建sqlex源码目录
        val sourceDirectory = project.objects.sourceDirectorySet(sourceSet.name, "${sourceSet.name} SqlEx source")
        //设置默认路径,可被配置修改
        sourceDirectory.srcDir("src/${sourceSet.name}/sqlex")
        //添加到源码集中
        sourceSet.extensions.add("sqlex", sourceDirectory)

        //给对应的源码集配置SqlEx编译任务
        val task =
            project.tasks.create(
                "generateSqlExSourceFor${sourceSet.name.pascalName}",
                GenerateSqlExTask::class.java
            ) {
                it.sqlexSourceDirs = sourceDirectory.srcDirs
                it.srcSet = sourceSet
                it.group = "sqlex"
                it.description = "SqlEx 源码生成任务 [${sourceSet.name}]"
            }

        //让java编译依赖于SqlEx编译任务
        project.tasks.withType(JavaCompile::class.java) { it.dependsOn(task) }

        project.tasks.findByName(sourceSet.getCompileTaskName("kotlin"))?.dependsOn(task)
        project.tasks.findByName(sourceSet.getCompileTaskName("groovy"))?.dependsOn(task)
        project.tasks.findByName(sourceSet.getCompileTaskName("scala"))?.dependsOn(task)
    }

    //依赖版本管理
    private fun configureDependencies() {
        project.configurations.forEach { config ->
            config.resolutionStrategy.eachDependency { resolveDetails ->
                val dep = resolveDetails.target
                if (dep.group == "me.danwi.sqlex") {
                    if (dep.version.isNullOrEmpty()) {
                        resolveDetails.useVersion(BuildFile.VERSION)
                    } else if (dep.version != BuildFile.VERSION) {
                        throw GradleException("Gradle插件版本和Core依赖版本不一致, Gradle Plugin: ${BuildFile.VERSION}, Core: ${dep.version}")
                    }
                }
            }
        }
    }

    //配置注解处理器
    private fun configureAnnotationProcessor() {
        val configName = "annotationProcessor"
        val apt = project.configurations.getByName(configName).dependencies
            .any { it.group == "me.danwi.sqlex" && it.name == "core" }
        if (!apt)
            project.dependencies.add(configName, "me.danwi.sqlex:core:${BuildFile.VERSION}")
    }
}
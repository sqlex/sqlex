package me.danwi.sqlex.gradle

import me.danwi.sqlex.core.Build
import me.danwi.sqlex.parser.util.pascalName
import org.gradle.api.GradleException
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.file.SourceDirectorySet
import org.gradle.api.plugins.JavaPlugin
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
            //配置源码集
            project.extensions.configure(JavaPluginExtension::class.java) {
                it.sourceSets.forEach(::configureSourceSet)
            }
            //注解处理器配置
            configureAnnotationProcessor()
            isApplied = true
        }

        project.afterEvaluate {
            if (!isApplied)
                throw GradleException("SqlEx插件依赖Java插件做代码编译,请在项目中引入Java插件")
            //配置生成任务
            project.extensions.configure(JavaPluginExtension::class.java) {
                it.sourceSets.forEach(::configureTask)
            }
            //检查依赖版本
            configureDependencies()
        }
    }

    //针对java的源码集生成对应的sqlex源码目录,并添加对应的编译任务
    private fun configureSourceSet(sourceSet: SourceSet) {
        //新建sqlex源码目录
        val sourceDirectory = project.objects.sourceDirectorySet(sourceSet.name, "${sourceSet.name} sqlex source")
        //设置默认路径,可被配置修改
        sourceDirectory.srcDir("src/${sourceSet.name}/sqlex")
        //添加到源码集中
        sourceSet.extensions.add("sqlex", sourceDirectory)
    }

    //给SqlEx的源码集配置对应的生成任务
    private fun configureTask(sourceSet: SourceSet) {
        val sourceDirectory = sourceSet.extensions.getByName("sqlex") as SourceDirectorySet
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

        //让资源处理依赖于SqlEx编译任务
        project.tasks.findByName(sourceSet.processResourcesTaskName)?.dependsOn(task)
    }

    //配置注解处理器
    private fun configureAnnotationProcessor() {
        //兼容kotlin,判断kapt插件是否存在
        if (project.pluginManager.hasPlugin("org.jetbrains.kotlin.kapt")) {
            //如果kapt插件存在,则添加kapt依赖
            project.dependencies.add("kapt", "me.danwi.sqlex:core:${Build.VERSION}")
            project.dependencies.add("kaptTest", "me.danwi.sqlex:core:${Build.VERSION}")
        } else {
            //如果kapt插件不存在,则添加普通的annotationProcessor依赖
            project.dependencies.add(
                JavaPlugin.ANNOTATION_PROCESSOR_CONFIGURATION_NAME,
                "me.danwi.sqlex:core:${Build.VERSION}"
            )
            project.dependencies.add(
                JavaPlugin.TEST_ANNOTATION_PROCESSOR_CONFIGURATION_NAME,
                "me.danwi.sqlex:core:${Build.VERSION}"
            )
        }
    }

    //依赖版本管理
    private fun configureDependencies() {
        val groupName = "me.danwi.sqlex"
        val artifactNames = listOf("core", "core-kotlin")
        //给没有标记版本的添加版本标记
        project.configurations.forEach { config ->
            config.resolutionStrategy.eachDependency { resolveDetails ->
                if (resolveDetails.target.group == groupName
                    && resolveDetails.target.name in artifactNames
                    && resolveDetails.target.version.isNullOrBlank()
                )
                    resolveDetails.useVersion(Build.VERSION)
            }
        }
        //检查是否有版本不匹配的情况
        project.configurations.flatMap { it.dependencies }
            .filter { it.group == groupName && it.name in artifactNames && !it.version.isNullOrEmpty() }
            .filter { it.version != Build.VERSION }
            .forEach { throw GradleException("Gradle插件版本和Core依赖版本不一致, Gradle Plugin: ${Build.VERSION}, Core: ${it.version}") }
    }
}
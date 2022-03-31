package me.danwi.sqlex.gradle

import org.gradle.api.GradleException
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.Task
import org.gradle.api.file.SourceDirectorySet
import org.gradle.api.tasks.SourceSetContainer
import org.gradle.plugins.ide.idea.IdeaPlugin
import org.gradle.plugins.ide.idea.model.IdeaModel

class SqlExPlugin implements Plugin<Project> {
    private Project project

    @Override
    void apply(Project project) {
        this.project = project

        //确保语言插件存在
        ensureCompileLanguagePlugin()
        //添加source set
        addSourceSetExtensions()

        project.afterEvaluate {
            //添加sqlex任务
            addSqlExTasks()
            //idea集成
            addSourcesToIDE()
        }
    }

    private SourceSetContainer getSourceSets() {
        return project.sourceSets
    }

    //确保有一个能编译jvm的语言插件,如果没有自己应用一个,用于编译生成的代码
    private void ensureCompileLanguagePlugin() {
        if (!project.pluginManager.hasPlugin('java'))
            throw new GradleException("项目没有引入Java插件")
    }

    //添加source set拓展
    private void addSourceSetExtensions() {
        getSourceSets().all { sourceSet ->
            String name = sourceSet.name
            SourceDirectorySet sds = project.objects.sourceDirectorySet(name, "${name} SqlEx source")
            sourceSet.extensions.add('sqlex', sds)
            sds.srcDir("src/${name}/sqlex")
        }
    }

    //添加源码生成任务
    private void addSqlExTasks() {
        getSourceSets().each { sourceSet ->
            String name = sourceSet.name
            SourceDirectorySet sqlexSrcDirSet = sourceSet.sqlex
            if (sqlexSrcDirSet != null) {
                Task task = project.tasks.create("generateSqlExSourceFor${name.charAt(0).toUpperCase()}${name.substring(1)}", GenerateSqlExTask) {
                    sqlexSourceDirs = sqlexSrcDirSet.srcDirs
                    srcSet = sourceSet
                    group = "sqlex"
                    description = "SqlEx源码生成任务 [${sourceSet.name}]"
                }

                Task javaCompileTask = project.tasks.findByName(sourceSet.getCompileTaskName("java"))
                if (javaCompileTask != null)
                    javaCompileTask.dependsOn(task)
                Task kotlinCompileTask = project.tasks.findByName(sourceSet.getCompileTaskName("kotlin"))
                if (kotlinCompileTask != null)
                    kotlinCompileTask.dependsOn(task)
            }
        }
    }

    //添加source set到 IDE
    private void addSourcesToIDE() {
        if (project.getExtensions().findByType(IdeaModel) == null)
            project.apply([plugin: IdeaPlugin])
        def model = project.getExtensions().findByType(IdeaModel)
        getSourceSets().each { sourceSet ->
            SourceDirectorySet sqlexSrcDirSet = sourceSet.sqlex
            sqlexSrcDirSet.srcDirs.each { sqlexDir ->
                if (sourceSet.name == "test")
                    model.module.testSourceDirs += sqlexDir
                else
                    model.module.sourceDirs += sqlexDir
            }
        }
    }
}

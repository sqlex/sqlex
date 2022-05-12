import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    `java-gradle-plugin`
    groovy
    kotlin("jvm")
    `maven-publish`
    id("com.gradle.plugin-publish") version "1.0.0-rc-1"
}

dependencies {
    implementation(project(":parser"))
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
}

//kotlin和groovy兼容交互
tasks {
    val compileKotlin = named("compileKotlin", KotlinCompile::class).get()
    val compileGroovy = named("compileGroovy", GroovyCompile::class).get()
    val classes by getting

    compileGroovy.dependsOn(compileKotlin)
    compileGroovy.classpath += files(compileKotlin.destinationDirectory)
    classes.dependsOn(compileGroovy)
}

gradlePlugin {
    plugins {
        create("sqlex") {
            id = "me.danwi.sqlex"
            implementationClass = "me.danwi.sqlex.gradle.SqlExPlugin"
            displayName = "Gradle SqlEx plugin"
            description = "Adds support for sqlex framework, compile sqlex repository to java code"
        }
    }
}

pluginBundle {
    website = "https://github.com/sqlex/sqlex/tree/master/gradle-plugin"
    vcsUrl = "https://github.com/sqlex/sqlex"
    tags = listOf("sqlex", "orm", "dbhelper")
}

//设置gradle上传的密钥
if (ext.has("gradle.key")) {
    tasks.create("setupPluginUpload") {
        System.setProperty("gradle.publish.key", ext["gradle.key"].toString())
        System.setProperty("gradle.publish.secret", ext["gradle.secret"].toString())
    }
    tasks.publishPlugins {
        dependsOn(tasks["setupPluginUpload"])
    }
}

//禁止当前构建被上传到maven中央仓库,因为其已经上传到gradle的插件仓库,see https://github.com/gradle-nexus/publish-plugin/issues/143
gradle.taskGraph.whenReady {
    tasks.withType<PublishToMavenRepository> {
        if (repository == null)
            logger.info("Task `{}` had null repository", path)
        else if (repository.name == "sonatype") {
            logger.lifecycle("Disabling task `{}` because it publishes to Sonatype", path)
            enabled = false
        }
    }
}
plugins {
    `java-gradle-plugin`
    kotlin("jvm")
    `maven-publish`
    id("com.gradle.plugin-publish") version "1.0.0-rc-1"
}

dependencies {
    implementation(project(":parser"))
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
}

//让程序能读取到gradle配置的版本
val generateBuildJava = tasks.create("generateBuildJava") {
    val outputDir = "$buildDir/generated/java"
    this.extra.set("outputDir", outputDir)
    inputs.property("version", project.version)
    outputs.dir(outputDir)
    doLast {
        mkdir("$outputDir/me/danwi/sqlex/gradle/")
        file("$outputDir/me/danwi/sqlex/gradle/BuildFile.java").writeText(
            """
            package me.danwi.sqlex.gradle;
            
            public class BuildFile {
                public static final String VERSION = "${project.version}";
            }
        """.trimIndent()
        )
    }
}
tasks.findByName("compileJava")?.dependsOn(generateBuildJava)
tasks.findByName("compileKotlin")?.dependsOn(generateBuildJava)
sourceSets { main { java { srcDir(generateBuildJava.extra.get("outputDir")!!) } } }

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

//gradle插件上传到中央仓库做特殊处理
afterEvaluate {
    publishing.publications.filterIsInstance<MavenPublication>().forEach {
        it.apply {
            pom {
                name.set("Gradle SqlEx plugin")
                description.set("Adds support for sqlex framework, compile sqlex repository to java code")
                url.set("https://github.com/sqlex")
                scm {
                    connection.set("scm:git:https://github.com/sqlex/sqlex.git")
                    developerConnection.set("scm:git:https://github.com/sqlex/sqlex.git")
                    url.set("https://github.com/sqlex")
                }
                licenses {
                    license {
                        name.set("The Apache License, Version 2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
                    }
                }
                developers {
                    developer {
                        id.set("sqlex")
                        name.set("sqlex")
                        email.set("demon@danwi.me")
                    }
                }
            }
        }
    }
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
plugins {
    kotlin("jvm") version "1.8.20" apply false
    id("io.github.gradle-nexus.publish-plugin") version "1.1.0"
}

//加载全局的local.properties
val globalProperties = java.util.Properties()
//判断是否为RELEASE环境
if (System.getenv("RELEASE") != null) {
    globalProperties["release"] = true
    globalProperties["development"] = false
} else {
    globalProperties["release"] = false
    globalProperties["development"] = true
}
val globalLocalFile = project.file("local.properties")
if (globalLocalFile.isFile)
    globalProperties.load(globalLocalFile.inputStream())
globalProperties.forEach { key, value ->
    ext.set(key as String, value)
}

//所有项目应用
allprojects {
    //本地属性相关
    val localProperties = globalProperties.clone() as java.util.Properties
    val localFile = this.file("local.properties")
    if (localFile.isFile)
        localProperties.load(localFile.inputStream())
    localProperties.forEach { key, value ->
        ext.set(key as String, value)
    }

    group = "me.danwi.sqlex"
    version = "0.14.0"

    //开发环境,版本统一添加SNAPSHOT
    if (ext["development"] == true)
        version = "$version-SNAPSHOT"

    repositories {
        mavenCentral()
    }

    //java相关配置
    pluginManager.withPlugin("java") {
        configure<JavaPluginExtension> {
            withJavadocJar()
            withSourcesJar()
        }

        tasks.withType<JavaCompile> {
            sourceCompatibility = "1.8"
            targetCompatibility = "1.8"
            options.encoding = "UTF-8"
        }

        tasks.withType<Javadoc> {
            options.encoding = "UTF-8"
        }
    }

    //kotlin相关配置
    pluginManager.withPlugin("org.jetbrains.kotlin.jvm") {
        tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
            kotlinOptions.jvmTarget = "1.8"
            kotlinOptions.freeCompilerArgs = listOf("-Xjvm-default=all-compatibility")
        }
    }

    tasks.withType<Test> {
        useJUnitPlatform()
    }

    afterEvaluate {
        //如果配置了maven-publish插件,且不是gradle插件项目(需要自定义发布),则自动填充相关配置
        if (pluginManager.hasPlugin("maven-publish") && !pluginManager.hasPlugin("java-gradle-plugin")) {
            configure<PublishingExtension> {
                publications {
                    create<MavenPublication>("Java") {
                        from(components["java"])
                        pom {
                            name.set(project.name)
                            description.set("sqlex ${project.name} component")
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
        }

        //如果配置了maven-publish,有签名配置,且没有应用自己的签名插件,则尝试应用全局签名配置
        if (pluginManager.hasPlugin("maven-publish")
            && localProperties.getProperty("signing.key") != null
            && !pluginManager.hasPlugin("signing")
        ) {
            pluginManager.apply("signing")
            configure<SigningExtension> {
                useInMemoryPgpKeys(
                    localProperties["signing.key"].toString(),
                    localProperties["signing.password"].toString()
                )
                extensions.configure<PublishingExtension> {
                    publications.forEach { sign(it) }
                }
            }
        }
    }
}

//配置全局的maven central属性
if (globalProperties.getProperty("ossrh.username") != null) {
    pluginManager.withPlugin("io.github.gradle-nexus.publish-plugin") {
        configure<io.github.gradlenexus.publishplugin.NexusPublishExtension> {
            repositories {
                sonatype {
                    username.set(globalProperties["ossrh.username"].toString())
                    password.set(globalProperties["ossrh.password"].toString())
                }
            }
        }
    }
}

plugins {
    java
    kotlin("jvm")
    id("org.jetbrains.intellij.platform") version "2.1.0"
}

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    implementation(project(":parser"))
    implementation("org.antlr:antlr4-intellij-adaptor:0.1")

    intellijPlatform {
        //平台依赖
        if (project.ext.has("idea.path")) {
            local(project.ext["idea.path"].toString())
        } else {
            intellijIdeaUltimate("2023.1")
        }
        //插件依赖
        bundledPlugin("org.jetbrains.plugins.yaml")
        bundledPlugin("com.intellij.java")
        bundledPlugin("com.intellij.database")
        bundledPlugin("org.jetbrains.kotlin")
        bundledPlugin("com.intellij.spring")
    }
}

intellijPlatform {
    projectName = "sqlex"

    buildSearchableOptions = false
    instrumentCode = false

    pluginConfiguration {
        id = "me.danwi.sqlex"
        name = "SqlEx"
        vendor {
            name = "SqlEx"
            email = "demon@danwi.me"
            url = "https://sqlex.github.io"
        }
        description = """
        Adds support for the <a href="https://sqlex.github.io">sqlex</a> framework(A data access framework).
        The following features are available:
        <ul>
            <li>Coding assistance(auto completion, formatting, highlighting...)</li>
            <li>Navigation between java and sqlm, provide symbol information</li>
            <li>Integration with Java/Kotlin/Spring</li>
            <li>Error inspection(such as incorrect grammar)</li>
        </ul>
        For more information, please see <a href="https://sqlex.github.io">sqlex official site</a>.
        """.trimIndent()
        ideaVersion {
            sinceBuild = "231"
            untilBuild = "243.*"
        }
    }

    if (project.ext.has("idea.token") && project.ext["release"] == true) {
        publishing {
            token = project.ext["idea.token"].toString()
        }
    }
}

tasks.runIde {
    //修改调试时候的IDEA的内存使用限制
    jvmArgs = listOf("-Xms1024m", "-Xmx2048m")
}
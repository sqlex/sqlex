plugins {
    java
    kotlin("jvm")
    id("org.jetbrains.intellij") version "1.11.0"
}

dependencies {
    implementation(project(":parser"))

    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("org.antlr:antlr4-intellij-adaptor:0.1")
}

// See https://github.com/JetBrains/gradle-intellij-plugin/
intellij {
    if (project.ext.has("idea.path")) {
        localPath.set(project.ext["idea.path"].toString())
    } else {
        type.set("IU")
        version.set("2022.3")
    }
    pluginName.set("sqlex")
    plugins.set(
        listOf(
            "org.jetbrains.plugins.yaml",
            "com.intellij.java",
            "com.intellij.database",
            "org.jetbrains.kotlin",
            "com.intellij.spring"
        )
    )
}

tasks.runIde {
    //修改调试时候的IDEA的内存使用限制
    jvmArgs = listOf("-Xms1024m", "-Xmx2048m")
}

tasks.patchPluginXml {
    sinceBuild.set("211")
    untilBuild.set("223.*")
}

if (ext.has("idea.token") && ext["release"] == true) {
    tasks.publishPlugin {
        token.set(ext["idea.token"].toString())
    }
}

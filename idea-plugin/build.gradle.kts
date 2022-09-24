plugins {
    java
    kotlin("jvm")
    id("org.jetbrains.intellij") version "1.9.0"
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
        version.set("2021.1")
    }
    pluginName.set("sqlex")
    plugins.set(listOf("yaml", "java", "DatabaseTools", "Kotlin", "Spring"))
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

plugins {
    java
    kotlin("jvm")
    id("org.jetbrains.intellij") version "1.5.2"
}

dependencies {
    implementation(project(":parser"))

    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("org.antlr:antlr4-intellij-adaptor:0.1")
}

// See https://github.com/JetBrains/gradle-intellij-plugin/
intellij {
    if (ext.has("idea.path")) {
        localPath.set(ext["idea.path"].toString())
    } else {
        type.set("IU")
        version.set("2021.1")
    }
    pluginName.set("sqlex")
    plugins.set(listOf("yaml", "java", "DatabaseTools", "Kotlin", "Spring"))
}

tasks.patchPluginXml {
    sinceBuild.set("211")
    untilBuild.set("221.*")
}

if (ext.has("idea.token") && ext["release"] == true) {
    tasks.publishPlugin {
        token.set(ext["idea.token"].toString())
    }
}

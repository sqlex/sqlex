plugins {
    java
    antlr
    kotlin("jvm")
    `maven-publish`
}

dependencies {
    api(project(":core"))
    implementation(project(":native-windows-amd64"))
    implementation(project(":native-linux-amd64"))
    implementation(project(":native-darwin-amd64"))
    implementation(project(":native-darwin-aarch64"))
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("net.java.dev.jna:jna:5.12.1")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.13.4")
    implementation("com.fasterxml.jackson.dataformat:jackson-dataformat-yaml:2.13.4")
    implementation("com.fasterxml.jackson.module:jackson-module-kotlin:2.13.4")
    implementation("com.squareup:javapoet:1.13.0")
    antlr("org.antlr:antlr4:4.9.2") {
        exclude("com.ibm.icu", "icu4j")
    }
    implementation("org.jetbrains:annotations:23.0.0")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.9.0")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.9.0")
}

tasks.generateGrammarSource {
    arguments.addAll(listOf("-package", "me.danwi.sqlex.parser", "-Xexact-output-dir"))

    doLast {
        val parserPackagePath = "${outputDirectory.canonicalPath}/me/danwi/sqlex/parser"

        file(parserPackagePath).mkdirs()

        copy {
            from(outputDirectory.canonicalPath)
            into(parserPackagePath)
            include("SqlEx*")
        }

        delete(fileTree(outputDirectory.canonicalPath) {
            include("SqlEx*")
        })
    }
}

tasks.compileKotlin {
    dependsOn(tasks.generateGrammarSource)
}

tasks.compileTestKotlin {
    dependsOn(tasks.generateTestGrammarSource)
}
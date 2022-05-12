plugins {
    id("java")
    id("antlr")
    kotlin("jvm")
    id("maven-publish")
}

dependencies {
    api(project(":core"))
    implementation("org.jetbrains.kotlin:kotlin-stdlib")
    implementation("net.java.dev.jna:jna:5.11.0")
    implementation("com.google.code.gson:gson:2.9.0")
    implementation("org.yaml:snakeyaml:1.30")
    implementation("com.squareup:javapoet:1.13.0")
    antlr("org.antlr:antlr4:4.10.1") {
        exclude("com.ibm.icu", "icu4j")
    }

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.8.2")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.8.2")
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

tasks.sourcesJar {
    dependsOn(tasks.classes)
    archiveClassifier.set("sources")
    from(sourceSets["main"].allSource)
    exclude("**/*.dylib")
    exclude("**/*.dll")
    exclude("**/*.so")
    exclude("**/*.h")
}
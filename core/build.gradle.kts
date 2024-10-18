plugins {
    java
    `java-library`
    `maven-publish`
}

//生成构建信息类
val generateBuildJava = tasks.create("generateBuildJava") {
    val outputDir = "$buildDir/generated/java"
    this.extra.set("outputDir", outputDir)
    inputs.property("version", project.version)
    outputs.dir(outputDir)
    doLast {
        mkdir("$outputDir/me/danwi/sqlex/core/")
        file("$outputDir/me/danwi/sqlex/core/Build.java").writeText(
            """
            package me.danwi.sqlex.core;
            
            public class Build {
                public static final String VERSION = "${project.version}";
            }
        """.trimIndent()
        )
    }
}
tasks.compileJava { dependsOn(generateBuildJava) }
tasks.sourcesJar { dependsOn(generateBuildJava) }
sourceSets { main { java { srcDir(generateBuildJava.extra.get("outputDir")!!) } } }

dependencies {
    implementation("org.slf4j:slf4j-api:1.7.36")
    implementation("org.apache.shardingsphere:shardingsphere-sql-parser-mysql:5.1.2")

    api("org.jetbrains:annotations:23.0.0")
    compileOnly("org.springframework:spring-jdbc:5.3.22")
    compileOnly("org.springframework:spring-tx:5.3.22")
    compileOnly("org.springframework:spring-context:5.3.22")
    compileOnly("mysql:mysql-connector-java:8.0.30")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.9.0")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.9.0")
}
plugins {
    java
    kotlin("jvm")
    `maven-publish`
    id("de.benediktritter.maven-plugin-development") version "0.4.3"
}

dependencies {
    implementation(project(":parser"))
    implementation("org.apache.maven:maven-plugin-api:3.8.5")
    compileOnly("org.apache.maven:maven-core:3.8.5")
    compileOnly("org.apache.maven.plugin-tools:maven-plugin-annotations:3.6.4")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.9.0")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.9.0")
}

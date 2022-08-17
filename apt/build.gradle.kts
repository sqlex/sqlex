plugins {
    java
    kotlin("jvm")
    `maven-publish`
}

dependencies {
    implementation(project(":core"))
}
plugins {
    kotlin("jvm")
    `maven-publish`
}

dependencies {
    compileOnly(project(":core"))
}

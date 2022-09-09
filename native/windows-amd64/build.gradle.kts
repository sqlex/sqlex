plugins {
    java
    `maven-publish`
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
plugins {
    java
    `java-library`
    `maven-publish`
}

dependencies {
    api("org.jetbrains:annotations:23.0.0")

    implementation("org.slf4j:slf4j-api:1.7.36")

    compileOnly("mysql:mysql-connector-java:8.0.30")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.9.0")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.9.0")
}
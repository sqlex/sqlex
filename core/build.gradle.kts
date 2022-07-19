plugins {
    java
    `java-library`
    `maven-publish`
}

dependencies {
    implementation("org.slf4j:slf4j-api:1.7.36")

    api("org.jetbrains:annotations:23.0.0")
    compileOnly("org.springframework:spring-jdbc:5.3.19")
    compileOnly("org.springframework:spring-tx:5.3.19")
    compileOnly("org.springframework:spring-context:5.3.19")
    compileOnly("mysql:mysql-connector-java:8.0.29")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.8.2")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.8.2")
}
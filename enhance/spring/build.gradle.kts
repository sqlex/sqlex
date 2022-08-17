plugins {
    java
    `java-library`
    `maven-publish`
}

dependencies {
    api(project(":core"))

    compileOnly("org.springframework:spring-jdbc:5.3.22")
    compileOnly("org.springframework:spring-tx:5.3.22")
    compileOnly("org.springframework:spring-context:5.3.22")
}
plugins {
    java
    `java-library`
    `maven-publish`
}

dependencies {
    implementation("org.slf4j:slf4j-api:1.7.36")
    implementation("com.alibaba:druid:1.2.11")

    api("org.jetbrains:annotations:23.0.0")
    compileOnly("org.springframework:spring-jdbc:5.3.22")
    compileOnly("org.springframework:spring-tx:5.3.22")
    compileOnly("org.springframework:spring-context:5.3.22")
    compileOnly("mysql:mysql-connector-java:8.0.30")

    testImplementation("org.junit.jupiter:junit-jupiter-api:5.9.0")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.9.0")
}
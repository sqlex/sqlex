rootProject.name = "sqlex"

include("core")
include("core-kotlin")
include("idea-plugin")
include("parser")
include("gradle-plugin")
include("maven-plugin")
project(":maven-plugin").name = "sqlex-maven-plugin"
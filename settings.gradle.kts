rootProject.name = "sqlex"

include("core")
include("idea-plugin")
include("parser")
include("gradle-plugin")
include("maven-plugin")
project(":maven-plugin").name = "sqlex-maven-plugin"
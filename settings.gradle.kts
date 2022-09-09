rootProject.name = "sqlex"

include("core")
include("core-kotlin")
include("idea-plugin")
include("parser")
include("gradle-plugin")
include("maven-plugin")
project(":maven-plugin").name = "sqlex-maven-plugin"

include("native-windows-amd64")
project(":native-windows-amd64").projectDir = file("native/windows-amd64")
include("native-linux-amd64")
project(":native-linux-amd64").projectDir = file("native/linux-amd64")
include("native-darwin-amd64")
project(":native-darwin-amd64").projectDir = file("native/darwin-amd64")
include("native-darwin-aarch64")
project(":native-darwin-aarch64").projectDir = file("native/darwin-aarch64")
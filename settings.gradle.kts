rootProject.name = "sqlex"

//核心模块
include("core")
include("parser")
include("compiler")

//集成增强模块
include("kotlin")
project(":kotlin").projectDir = file("enhance/kotlin")
include("spring")
project(":spring").projectDir = file("enhance/spring")

//插件模块
include("idea-plugin")
project(":idea-plugin").projectDir = file("plugins/idea-plugin")
include("gradle-plugin")
project(":gradle-plugin").projectDir = file("plugins/gradle-plugin")
include("maven-plugin")
project(":maven-plugin").projectDir = file("plugins/maven-plugin")
project(":maven-plugin").name = "sqlex-maven-plugin"

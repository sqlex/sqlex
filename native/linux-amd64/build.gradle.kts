import java.nio.file.Paths

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

val splitTask = tasks.create("splitDynamicLibrary") {
    outputs.upToDateWhen { false }
    doLast {
        val archDirPath = "native/linux/amd64"
        val libFileName = "libsqlex.so"
        //创建资源输出目录
        val resourceDir = sourceSets.main.get().output.resourcesDir ?: throw GradleException("无法获取资源输出目录")
        resourceDir.mkdirs()
        //读取库文件
        val libFile =
            Paths.get(projectDir.absolutePath, "src/main/resources", archDirPath, libFileName).toFile()
        if (!libFile.exists()) {
            logger.warn("原生库文件不存在")
            return@doLast
        }
        val libFileInputStream = libFile.inputStream()
        libFileInputStream.use {
            //1M一个分片
            val buf = ByteArray(1024 * 1024)
            //准备分片目录
            val segmentDir = Paths.get(resourceDir.absolutePath, archDirPath, "$libFileName.d").toFile()
            segmentDir.deleteRecursively()
            segmentDir.mkdirs()
            //循环释放分片
            var index = 0
            while (true) {
                val n = libFileInputStream.read(buf)
                if (n == -1)
                    break
                //写入片段信息
                Paths.get(segmentDir.absolutePath, index.toString()).toFile()
                    .writeBytes(buf.sliceArray(0 until n))
                index++
            }
            //写入分片信息
            Paths.get(segmentDir.absolutePath, "max.id").toFile().writeText((index - 1).toString())
        }
    }
}

tasks.processResources {
    outputs.upToDateWhen { false }
    dependsOn(splitTask)
    exclude("**/native/**")
}
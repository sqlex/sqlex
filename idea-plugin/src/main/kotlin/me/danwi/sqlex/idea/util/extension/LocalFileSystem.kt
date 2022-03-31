package me.danwi.sqlex.idea.util.extension

import java.io.File
import java.io.IOException

//写入本地文件
fun writeLocalFile(path: String, content: String): File {
    val file = File(path)
    //删除旧文件
    try {
        file.delete()
    } catch (_: Exception) {
    }
    //确保父目录存在
    file.parentFile.mkdirs()
    //创建文件,并写入
    if (!file.createNewFile())
        throw IOException("无法创建新的文件")
    file.writeText(content)
    return file
}
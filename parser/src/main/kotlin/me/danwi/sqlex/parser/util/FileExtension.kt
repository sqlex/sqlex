package me.danwi.sqlex.parser.util

import me.danwi.sqlex.core.Build
import me.danwi.sqlex.parser.exception.SqlExRepositoryException
import java.io.File
import java.nio.file.Paths
import java.security.MessageDigest

//缓存hash文件后缀
private val SelfCacheHashExt = "self.cache"
private val SourceCacheHashExt = "source.cache"

//获取目录的缓存hash
var File.selfCacheHash: String?
    get() {
        return try {
            if (!this.isDirectory)
                return null
            Paths.get(this.parentFile.absolutePath, "${this.nameWithoutExtension}.${SelfCacheHashExt}")
                .toFile()
                .readText()
        } catch (_: Exception) {
            null
        }
    }
    set(value) {
        if (value == null)
            return
        Paths.get(this.parentFile.absolutePath, "${this.nameWithoutExtension}.${SelfCacheHashExt}")
            .toFile()
            .writeText(value)
    }

//获取目录对应的源码缓存hash
var File.sourceCacheHash: String?
    get() {
        return try {
            if (!this.isDirectory)
                return null
            Paths.get(this.parentFile.absolutePath, "${this.nameWithoutExtension}.${SourceCacheHashExt}")
                .toFile()
                .readText()
        } catch (_: Exception) {
            null
        }
    }
    set(value) {
        if (value == null)
            return
        Paths.get(this.parentFile.absolutePath, "${this.nameWithoutExtension}.${SourceCacheHashExt}")
            .toFile()
            .writeText(value)
    }

//计算目录的缓存hash
val File.computeCacheHash: String
    get() {
        if (!this.isDirectory)
            throw SqlExRepositoryException("路径不是一个文件夹: $absolutePath")
        //遍历整个目录,并获取到所有的文件
        val allFiles = this.walk()
            .filter { it.name != SelfCacheHashExt && it.name != SourceCacheHashExt } //排除hash缓存文件
            .filter { it.isFile } //比如是文件
            .sortedBy { it.absolutePath } //按照路径排序
        //计算md5
        val digest = MessageDigest.getInstance("MD5")
        //更新程序版本
        digest.update(Build.VERSION.toByteArray())
        //读取文件
        val buf = ByteArray(1024)
        for (file in allFiles) {
            //更新文件路径
            digest.update(file.name.toByteArray())
            file.inputStream().use {
                while (true) {
                    val n = it.read(buf)
                    if (n == -1)
                        break
                    digest.update(buf, 0, n)
                }
            }
        }
        //编码
        return digest.digest().joinToString("") { String.format("%02x", it) }
    }
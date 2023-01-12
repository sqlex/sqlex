@file:JvmName("FFIInvoker")

package me.danwi.sqlex.parser.ffi

import com.fasterxml.jackson.databind.type.TypeFactory
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import me.danwi.sqlex.parser.exception.SqlExFFIException
import me.danwi.sqlex.parser.exception.SqlExFFIInvokeException
import java.io.File
import java.nio.charset.Charset
import java.security.MessageDigest

// 释放并加载原生库
fun loadNativeLibrary() {
    //释放原生动态库
    val resourcePath: String
    val dylibExtensionName: String
    //根据操作系统来选择
    val osName = System.getProperty("os.name")
    if (osName.startsWith("Mac OS")) {
        resourcePath = if (System.getProperty("os.arch") == "aarch64")
            "native/darwin/aarch64/libsqlex.dylib.d"
        else
            "native/darwin/amd64/libsqlex.dylib.d"
        dylibExtensionName = "dylib"
    } else if (osName.startsWith("Windows")) {
        resourcePath = "native/windows/amd64/libsqlex.dll.d"
        dylibExtensionName = "dll"
    } else {
        resourcePath = "native/linux/amd64/libsqlex.so.d"
        dylibExtensionName = "so"
    }
    val classLoader = object {}::class.java.classLoader
    //获取最大的分片ID
    val maxIdInputStream = classLoader.getResourceAsStream("$resourcePath/max.id")
        ?: throw SqlExFFIException("无法获取原生库信息")
    val maxId = maxIdInputStream.readBytes().toString(Charset.defaultCharset()).toInt()
    //组合计算分片的hash
    val digest = MessageDigest.getInstance("MD5")
    val buffer = ByteArray(1024)
    for (id in 0..maxId) {
        //分片流
        val segmentInputStream =
            classLoader.getResourceAsStream("$resourcePath/$id") ?: throw SqlExFFIException("无法读取分片信息")
        segmentInputStream.use {
            while (true) {
                val n = segmentInputStream.read(buffer)
                if (n == -1)
                    break
                digest.update(buffer, 0, n)
            }
        }
    }
    //计算出文件hash名称
    val hashFileName = "${digest.digest().joinToString("") { String.format("%02x", it) }}.$dylibExtensionName"
    //准备释放最终文件
    //创建临时目录
    val tempDir = System.getProperty("java.io.tmpdir")
    val dylibDir = File(tempDir, "sqlex_dylib")
    dylibDir.mkdirs()
    //最终动态库文件
    val dylibFile = File(dylibDir, hashFileName)
    //判断文件是否已经存在
    if (!dylibFile.exists()) {
        val dylibFileOutputStream = dylibFile.outputStream()
        dylibFileOutputStream.use {
            //组合释放分片
            for (id in 0..maxId) {
                val segmentInputStream =
                    classLoader.getResourceAsStream("$resourcePath/$id") ?: throw SqlExFFIException("无法读取分片信息")
                segmentInputStream.use {
                    while (true) {
                        val n = segmentInputStream.read(buffer)
                        if (n == -1)
                            break
                        dylibFileOutputStream.write(buffer, 0, n)
                    }
                }
            }
        }
    }
    //释放完毕,开始加载,并处理好可能存在的重复加载问题
    try {
        System.load(dylibFile.absolutePath)
    } catch (e: UnsatisfiedLinkError) {
        //忽略已经加载异常
        //TODO: 硬编码判断 UnsatisfiedLinkError 异常类型
        if (e.message?.contains("already loaded in another classloader") != true) {
            //rethrow
            throw e
        }
    }
}

data class FFIRequest(val moduleName: String, val methodName: String, val parameters: Array<out Any>)

data class FFIResponse<T>(val success: Boolean, val message: String, val returnValue: T)

val jacksonMapper = jacksonObjectMapper()

inline fun <reified T> ffiInvoke(module: String, method: String, vararg args: Any): T {
    //准备请求
    val request = FFIRequest(module, method, args)
    val requestJson = jacksonMapper.writeValueAsString(request)
    //调用
    val responseJson =
        NativeFFI.request(requestJson) ?: throw SqlExFFIInvokeException(module, method, "FFI调用空引用异常")
    //判断是否发生了错误
    if (responseJson == "") throw SqlExFFIInvokeException(module, method, "FFI调用发生错误")
    //解析结果
    val dataType = TypeFactory.defaultInstance().constructType(T::class.java)
    val responseType = TypeFactory.defaultInstance().constructParametricType(FFIResponse::class.java, dataType)
    val response: FFIResponse<T> = jacksonMapper.readValue(responseJson, responseType)
    //是否出错
    if (!response.success) throw SqlExFFIInvokeException(module, method, response.message)
    //解析返回
    return response.returnValue
}

fun ffiCall(module: String, method: String, vararg args: Any) {
    return ffiInvoke(module, method, *args)
}
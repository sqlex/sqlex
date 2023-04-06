@file:JvmName("FFIInvoker")

package me.danwi.sqlex.parser.ffi

import com.fasterxml.jackson.databind.type.TypeFactory
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import me.danwi.sqlex.parser.exception.SqlExFFIException
import me.danwi.sqlex.parser.exception.SqlExFFIInvokeException
import net.bytebuddy.dynamic.loading.ClassInjector
import java.io.File
import java.lang.reflect.Method
import java.nio.charset.Charset
import java.security.MessageDigest

// 释放并加载原生库
fun extractNativeLibrary(): String {
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
    return dylibFile.absolutePath
}

// 原生请求方法
val ffiRequestMethod: Method by lazy {
    //常量定义
    val nativeLibClassName = "me.danwi.sqlex.parser.ffi.NativeLib"
    val nativeFFIClassName = "me.danwi.sqlex.parser.ffi.NativeFFI"
    val nativeLibResourcePath = "me/danwi/sqlex/parser/ffi/NativeLib.bin"
    val nativeFFIResourcePath = "me/danwi/sqlex/parser/ffi/NativeFFI.bin"
    //最顶层的类加载器
    val systemLoader = ClassLoader.getSystemClassLoader()
    val (nativeLibClass, nativeFFIClass) = try {
        //先尝试使用最顶层的加载器加载
        Pair(
            systemLoader.loadClass(nativeLibClassName),
            systemLoader.loadClass(nativeFFIClassName)
        )
    } catch (_: Exception) {
        try {
            //如果最顶层的加载器无法加载,则尝试将字节码注入到最顶层的加载器中
            //获取字节码
            val nativeLibByteCodeStream =
                object {}::class.java.classLoader.getResourceAsStream(nativeLibResourcePath)!!
            val nativeFFIByteCodeStream =
                object {}::class.java.classLoader.getResourceAsStream(nativeFFIResourcePath)!!
            val nativeLibByteCode = nativeLibByteCodeStream.use { it.readBytes() }
            val nativeFFIByteCode = nativeFFIByteCodeStream.use { it.readBytes() }
            //类注入器
            val classInjector = ClassInjector.UsingReflection.ofSystemClassLoader()
            classInjector.injectRaw(
                mapOf(
                    nativeLibClassName to nativeLibByteCode,
                    nativeFFIClassName to nativeFFIByteCode
                )
            )
            val nativeLibClass = systemLoader.loadClass(nativeLibClassName)
            val nativeFFIClass = systemLoader.loadClass(nativeFFIClassName)
            Pair(nativeLibClass, nativeFFIClass)
        } catch (_: Exception) {
            //如果注入失败,则使用当前的类加载器直接反射加载
            Pair(Class.forName(nativeLibClassName), Class.forName(nativeFFIClassName))
        }
    }
    //先释放原生库
    val dylibFilePath = extractNativeLibrary()
    //设置全局变量
    nativeLibClass.getDeclaredField("dylibFilePath").set(null, dylibFilePath)
    //返回ffi request方法
    nativeFFIClass.getDeclaredMethod("request", String::class.java)
}

data class FFIRequest(val moduleName: String, val methodName: String, val parameters: Array<out Any>)

data class FFIResponse<T>(val success: Boolean, val message: String, val returnValue: T)

val jacksonMapper = jacksonObjectMapper()

inline fun <reified T> ffiInvoke(module: String, method: String, vararg args: Any): T {
    //准备请求
    val request = FFIRequest(module, method, args)
    val requestJson = jacksonMapper.writeValueAsString(request)
    //调用
    val responseJson = ffiRequestMethod.invoke(null, requestJson) as String
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
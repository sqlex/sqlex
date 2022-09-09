package me.danwi.sqlex.parser.ffi

import com.fasterxml.jackson.databind.type.TypeFactory
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import me.danwi.sqlex.parser.exception.SqlExFFIException
import me.danwi.sqlex.parser.exception.SqlExFFIInvokeException
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

interface FFIInterface : Library {
    fun FFIInvoke(reqJson: GoString.ByValue): Pointer?
}

data class FFIRequest(val moduleName: String, val methodName: String, val parameters: Array<out Any>)

data class FFIResponse<T>(val success: Boolean, val message: String, val returnValue: T)

val ffiInterface: FFIInterface by lazy {
    //释放原生动态库
    val resourcePath: String
    val dylibExtensionName: String
    //根据操作系统来选择
    val osName = System.getProperty("os.name")
    if (osName.startsWith("Mac OS")) {
        resourcePath = if (System.getProperty("os.arch") == "aarch64")
            "native/darwin/aarch64/libsqlex.dylib"
        else
            "native/darwin/amd64/libsqlex.dylib"
        dylibExtensionName = "dylib"
    } else if (osName.startsWith("Windows")) {
        resourcePath = "native/windows/amd64/libsqlex.dll"
        dylibExtensionName = "dll"
    } else {
        resourcePath = "native/linux/amd64/libsqlex.so"
        dylibExtensionName = "so"
    }
    //获取资源
    val hashInputStream =
        object {}::class.java.classLoader.getResourceAsStream(resourcePath)
            ?: throw SqlExFFIException("无法获取内嵌原生库")
    //释放后的名称
    val hashFileName = hashInputStream.use {
        //创建消息摘要
        val digest = MessageDigest.getInstance("MD5")
        val buffer = ByteArray(1024)
        var length: Int
        while (true) {
            length = hashInputStream.read(buffer)
            if (length == -1)
                break
            digest.update(buffer, 0, length)
        }
        val hash = digest.digest().joinToString("") { String.format("%02x", it) }
        "$hash.$dylibExtensionName"
    }
    //创建临时目录
    val tempDir = System.getProperty("java.io.tmpdir")
    val dylibDir = File(tempDir, "sqlex_dylib")
    dylibDir.mkdirs()
    //最终动态库文件
    val dylibFile = File(dylibDir, hashFileName)
    //判断文件是否已经存在
    if (!dylibFile.exists()) {
        //获取资源
        val dylibInputStream =
            object {}::class.java.classLoader.getResourceAsStream(resourcePath)
                ?: throw SqlExFFIException("无法获取内嵌原生库")
        //释放文件
        dylibInputStream.use {
            Files.copy(dylibInputStream, dylibFile.toPath(), StandardCopyOption.REPLACE_EXISTING)
        }
    }
    //释放完毕,开始加载
    Native.load(dylibFile.absolutePath, FFIInterface::class.java, mapOf(Pair(Library.OPTION_STRING_ENCODING, "utf-8")))
}

val jacksonMapper = jacksonObjectMapper()

inline fun <reified T> ffiInvoke(module: String, method: String, vararg args: Any): T {
    //准备请求
    val request = FFIRequest(module, method, args)
    val requestGoString = GoString.ByValue(jacksonMapper.writeValueAsString(request))
    //调用
    val responseJsonStrPointer =
        ffiInterface.FFIInvoke(requestGoString) ?: throw SqlExFFIInvokeException(module, method, "FFI调用空引用异常")
    //将返回值转换成字符串
    val responseJson = responseJsonStrPointer.getString(0, "utf-8")
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
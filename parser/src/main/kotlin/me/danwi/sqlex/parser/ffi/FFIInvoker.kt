package me.danwi.sqlex.parser.ffi

import com.google.gson.Gson
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

data class FFIRequest(val moduleName: String, val methodName: String, val params: Array<String>)

class FFIResponse {
    var success: Boolean = false
    lateinit var message: String
    lateinit var returnValue: String
}

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
            "native/darwin/x86/libsqlex.dylib"
        dylibExtensionName = "dylib"
    } else if (osName.startsWith("Windows")) {
        resourcePath = "native/windows/libsqlex.dll"
        dylibExtensionName = "dll"
    } else {
        resourcePath = "native/linux/libsqlex.so"
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

inline fun <reified T> ffiInvoke(module: String, method: String, vararg args: Any): T {
    //准备请求
    val request = FFIRequest(module, method, args.map { Gson().toJson(it) }.toTypedArray())
    val requestGoString = GoString.ByValue(Gson().toJson(request))
    //调用
    val responseJsonStrPointer =
        ffiInterface.FFIInvoke(requestGoString) ?: throw SqlExFFIInvokeException(module, method, "FFI调用空引用异常")
    //将返回值转换成字符串
    val responseJson = responseJsonStrPointer.getString(0, "utf-8")
    //判断是否发生了错误
    if (responseJson == "") throw SqlExFFIInvokeException(module, method, "FFI调用发生错误")
    //解析结果
    val response = Gson().fromJson(responseJson, FFIResponse::class.java)
    //是否出错
    if (!response.success) throw SqlExFFIInvokeException(module, method, response.message)
    //解析返回
    return Gson().fromJson(response.returnValue, T::class.java)
}

fun ffiCall(module: String, method: String, vararg args: Any) {
    return ffiInvoke(module, method, *args)
}
package me.danwi.sqlex.parser.ffi

import com.google.gson.Gson
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.nio.file.StandardCopyOption

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
    val dylibName: String
    //根据操作系统来选择
    val osName = System.getProperty("os.name")
    if (osName.startsWith("Mac OS")) {
        resourcePath = if (System.getProperty("os.arch") == "aarch64")
            "native/darwin/aarch64"
        else
            "native/darwin/x86"
        dylibName = "libsqlex.dylib"
    } else if (osName.startsWith("Windows")) {
        resourcePath = "native/windows"
        dylibName = "libsqlex.dll"
    } else {
        resourcePath = "native/linux"
        dylibName = "libsqlex.so"
    }
    //获取资源
    val inputStream =
        object {}::class.java.classLoader.getResourceAsStream("$resourcePath/$dylibName")
            ?: throw IOException("无法获取内嵌原生库")
    //创建临时文件
    val tempDir = System.getProperty("java.io.tmpdir")
    val dylibDir = File(tempDir, "sqlex_dylib_${System.nanoTime()}")
    if (!dylibDir.mkdirs()) throw IOException("无法创建原生库目录")
    dylibDir.deleteOnExit()
    val dylibFile = File(dylibDir, dylibName)
    //释放文件
    try {
        Files.copy(inputStream, dylibFile.toPath(), StandardCopyOption.REPLACE_EXISTING)
    } catch (e: Exception) {
        dylibFile.delete()
        throw e
    }
    dylibFile.deleteOnExit()
    //释放完毕,开始加载
    Native.load(dylibFile.absolutePath, FFIInterface::class.java, mapOf(Pair(Library.OPTION_STRING_ENCODING, "utf-8")))
}

inline fun <reified T> ffiInvoke(module: String, method: String, vararg args: Any): T {
    //准备请求
    val request = FFIRequest(module, method, args.map { Gson().toJson(it) }.toTypedArray())
    val requestGoString = GoString.ByValue(Gson().toJson(request))
    //调用
    val responseJsonStrPointer = ffiInterface.FFIInvoke(requestGoString) ?: throw Exception("FFI调用空引用异常")
    //将返回值转换成字符串
    val responseJson = responseJsonStrPointer.getString(0, "utf-8")
    //判断是否发生了错误
    if (responseJson == "") throw Exception("FFI调用发生错误")
    //解析结果
    val response = Gson().fromJson(responseJson, FFIResponse::class.java)
    //是否出错
    if (!response.success) throw Exception(response.message)
    //解析返回
    return Gson().fromJson(response.returnValue, T::class.java)
}

fun ffiCall(module: String, method: String, vararg args: Any) {
    return ffiInvoke(module, method, *args)
}
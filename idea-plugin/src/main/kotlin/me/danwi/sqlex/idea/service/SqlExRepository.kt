package me.danwi.sqlex.idea.service

import com.intellij.ide.highlighter.JavaFileType
import com.intellij.openapi.application.invokeLater
import com.intellij.openapi.application.runReadAction
import com.intellij.openapi.application.runWriteAction
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiFileFactory
import com.intellij.psi.PsiJavaFile
import com.intellij.psi.PsiManager
import me.danwi.sqlex.idea.util.extension.qualifiedPackageName
import me.danwi.sqlex.idea.util.extension.sourceRootRelativePath
import me.danwi.sqlex.idea.util.extension.textContent
import me.danwi.sqlex.parser.JavaFile
import me.danwi.sqlex.parser.Repository

val sqlexMethodGeneratedCacheKey = Key<Boolean>("me.danwi.sqlex.sqlexMethodGeneratedCache")
val sqlexMethodFileCacheKey = Key<VirtualFile>("me.danwi.sqlex.sqlexMethodFileCache")
val sqlexMethodPsiClassCacheKey = Key<PsiClass>("me.danwi.sqlex.sqlexMethodPsiClassCache")

class SqlExRepository(private val project: Project, private val repository: Repository) {
    private val javaFileCache = mutableMapOf<String, JavaFile>()
    private val javaClassCache = mutableMapOf<String, PsiClass>()
    private val psiManager = PsiManager.getInstance(project)

    init {
        //添加顶级Repository类
        val javaFile = repository.repositoryJavaFile
        //生成psi class
        val javaClass = runReadAction {
            val psiFile = PsiFileFactory.getInstance(project)
                .createFileFromText(
                    "${javaFile.className}.java",
                    JavaFileType.INSTANCE,
                    javaFile.source
                ) as PsiJavaFile
            psiFile.putUserData(sqlexMethodGeneratedCacheKey, true)
            psiFile.classes.firstOrNull()
        } ?: throw Exception("无法生成java class")
        //存入缓存
        javaClassCache["FAKE:${javaFile.relativePath}"] = javaClass
    }

    val allJavaClassCache: List<PsiClass>
        get() {
            val classes = javaClassCache.values
            val innerClass = classes.flatMap { it.innerClasses.toList() }
            return (classes + innerClass)
        }

    private fun generateJavaFile(file: VirtualFile): JavaFile {
        return repository.generateJavaFile(
            file.sourceRootRelativePath ?: throw Exception("无法获取文件${file.name}的相对路径"),
            file.textContent ?: throw Exception("无法读取文件${file.name}的内容")
        )
    }

    fun updateMethodFile(file: VirtualFile?) {
        if (file == null)
            return
        //生成java源码
        val javaFile = generateJavaFile(file)
        //生成psi class
        val javaClass = runReadAction {
            val psiFile = PsiFileFactory.getInstance(project)
                .createFileFromText(
                    "${javaFile.className}.java",
                    JavaFileType.INSTANCE,
                    javaFile.source
                ) as PsiJavaFile
            psiFile.putUserData(sqlexMethodGeneratedCacheKey, true)
            psiFile.classes.firstOrNull()
        } ?: throw Exception("无法生成java class")
        //放入virtual file缓存
        javaClass.putUserData(sqlexMethodFileCacheKey, file)
        file.putUserData(sqlexMethodPsiClassCacheKey, javaClass)
        //存入缓存
        javaClassCache[file.path] = javaClass
        //更新psi缓存
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    fun removeMethodFile(relativePath: String) {
        javaClassCache.remove(relativePath)
        javaFileCache.remove(relativePath)
        //更新缓存
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    fun removeMethodFile(file: VirtualFile?) {
        removeMethodFile(file?.path ?: return)
    }

    fun close() {
        javaFileCache.clear()
        javaClassCache.clear()
        repository.close()
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    //获取sqlm文件对应的java源码
    fun findJavaSource(file: VirtualFile?): JavaFile? {
        val path = file?.path ?: return null
        if (!javaFileCache.contains(path)) {
            javaFileCache[path] = generateJavaFile(file)
        }
        return javaFileCache[path]
    }

    //获取该包下所有的类(不包括内部类)
    fun findClasses(qualifiedPackage: String): List<PsiClass> {
        return javaClassCache.values
            .filter { it.qualifiedPackageName == qualifiedPackage }
    }

    //通过全限定名查找类
    fun findClass(qualifierName: String): PsiClass? {
        return allJavaClassCache.firstOrNull { it.qualifiedName == qualifierName }
    }

    //通过类名查找类
    fun findClassByName(className: String): PsiClass? {
        return allJavaClassCache.firstOrNull { it.name == className }
    }
}
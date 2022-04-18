package me.danwi.sqlex.idea.service

import com.intellij.ide.highlighter.JavaFileType
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiFileFactory
import com.intellij.psi.PsiJavaFile
import com.intellij.psi.PsiManager
import me.danwi.sqlex.idea.util.extension.*
import me.danwi.sqlex.parser.Repository
import me.danwi.sqlex.parser.generate.GeneratedJavaFile

val SqlExMethodGeneratedCacheKey = Key<Boolean>("me.danwi.sqlex.SqlExMethodGeneratedCache")
val SqlExMethodFileCacheKey = Key<VirtualFile>("me.danwi.sqlex.SqlExMethodFileCache")
val SqlExMethodPsiClassCacheKey = Key<PsiClass>("me.danwi.sqlex.SqlExMethodPsiClassCache")

class SqlExRepository(private val project: Project, private val repository: Repository) {
    private val javaFileCache = mutableMapOf<String, GeneratedJavaFile>()
    private val javaClassCache = mutableMapOf<String, PsiClass>()
    private val psiManager = PsiManager.getInstance(project)

    init {
        //生成顶级Repository的psi class
        val javaClass = generateJavaPsiClass(repository.repositoryJavaFile)
        //存入缓存
        javaClassCache["FAKE:${repository.repositoryJavaFile.relativePath}"] = javaClass
    }

    val allJavaClassCache: List<PsiClass>
        get() {
            val classes = javaClassCache.values
            val innerClass = classes.flatMap { it.innerClasses.toList() }
            return (classes + innerClass)
        }

    private fun generateJavaFile(file: VirtualFile): GeneratedJavaFile {
        return repository.generateJavaFile(
            file.sourceRootRelativePath ?: throw Exception("无法获取文件${file.name}的相对路径"),
            file.textContent ?: throw Exception("无法读取文件${file.name}的内容")
        )
    }

    private fun generateJavaPsiClass(generatedJavaSourceFile: GeneratedJavaFile): PsiClass {
        return runReadAction {
            val psiFile = PsiFileFactory.getInstance(project)
                .createFileFromText(
                    "${generatedJavaSourceFile.className}.java",
                    JavaFileType.INSTANCE,
                    generatedJavaSourceFile.source
                ) as PsiJavaFile
            //生成的virtual file可能不存在,需要通过view provider获取
            val generatedVirtualFile = psiFile.virtualFile ?: psiFile.viewProvider.virtualFile
            generatedVirtualFile.putUserData(SqlExMethodGeneratedCacheKey, true)
            psiFile.classes.firstOrNull()
        } ?: throw Exception("无法生成java class")
    }

    fun updateMethodFile(file: VirtualFile?) {
        if (file == null)
            return
        //生成java源码
        val javaFile = generateJavaFile(file)
        //生成psi class
        val javaClass = generateJavaPsiClass(javaFile)
        //放入virtual file缓存
        javaClass.putUserData(SqlExMethodFileCacheKey, file)
        file.putUserData(SqlExMethodPsiClassCacheKey, javaClass)
        //存入缓存
        javaClassCache[file.path] = javaClass
        //移除源码缓存
        javaFileCache.remove(file.path)
        //更新psi缓存
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    fun removeMethodFile(filePath: String) {
        javaClassCache.remove(filePath)
        javaFileCache.remove(filePath)
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

    //获取文件对应的java源码
    fun findJavaSource(file: VirtualFile?): GeneratedJavaFile? {
        //如果是配置文件,则返回顶级Repository的源码
        if (file.isSqlExConfig)
            return repository.repositoryJavaFile
        //其他则返回方法的源码
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
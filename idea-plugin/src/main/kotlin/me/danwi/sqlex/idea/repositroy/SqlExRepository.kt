package me.danwi.sqlex.idea.repositroy

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
    val session
        get() = repository.session

    private var repositoryJavaFile: GeneratedJavaFile? = null
    private var repositoryJavaClass: PsiClass? = null

    private val methodJavaFileCache = mutableMapOf<String, GeneratedJavaFile>()
    private val methodJavaClassCache = mutableMapOf<String, PsiClass>()
    private val psiManager = PsiManager.getInstance(project)

    private val repositoryJavaFileCache: GeneratedJavaFile
        get() {
            val file = repositoryJavaFile
                ?: repository.generateRepositoryClassFile(methodJavaFileCache.values.map { it.qualifiedName })
            if (repositoryJavaFile == null)
                repositoryJavaFile = file
            return file
        }

    private val repositoryJavaClassCache: PsiClass
        get() {
            val psiClass = repositoryJavaClass ?: generateJavaPsiClass(repositoryJavaFileCache)
            if (repositoryJavaClass == null)
                repositoryJavaClass = psiClass
            return psiClass
        }

    val allJavaClassCache: List<PsiClass>
        get() {
            val classes = methodJavaClassCache.values
            val innerClass = classes.flatMap { it.innerClasses.toList() }
            return (classes + innerClass + repositoryJavaClassCache)
        }

    private fun generateJavaFile(file: VirtualFile): GeneratedJavaFile {
        return repository.generateMethodClassFile(
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
        methodJavaFileCache[file.path] = javaFile
        methodJavaClassCache[file.path] = javaClass
        //移除repository缓存
        repositoryJavaFile = null
        repositoryJavaClass = null
        //更新psi缓存
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    fun removeMethodFile(filePath: String) {
        //移除method缓存
        methodJavaClassCache.remove(filePath)
        methodJavaFileCache.remove(filePath)
        //移除repository缓存
        repositoryJavaFile = null
        repositoryJavaClass = null
        //更新缓存
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    fun removeMethodFile(file: VirtualFile?) {
        removeMethodFile(file?.path ?: return)
    }

    fun close() {
        methodJavaFileCache.clear()
        methodJavaClassCache.clear()
        repository.close()
        invokeLater { runWriteAction { psiManager.dropPsiCaches() } }
    }

    //获取文件对应的java源码
    fun findJavaSource(file: VirtualFile?): GeneratedJavaFile? {
        //如果是配置文件,则返回顶级Repository的源码
        if (file.isSqlExConfig)
            return repositoryJavaFileCache
        //其他则返回方法的源码
        val path = file?.path ?: return null
        return methodJavaFileCache[path]
    }

    //获取该包下所有的类(不包括内部类)
    fun findClasses(qualifiedPackage: String): List<PsiClass> {
        return methodJavaClassCache.values
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
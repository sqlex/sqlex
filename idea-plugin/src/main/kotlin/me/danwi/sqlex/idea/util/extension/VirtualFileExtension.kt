package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.module.Module
import com.intellij.openapi.project.Project
import com.intellij.openapi.project.ProjectLocator
import com.intellij.openapi.roots.ProjectRootManager
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager
import me.danwi.sqlex.idea.service.SqlExRepositoryService
import me.danwi.sqlex.parser.util.SqlExConfigFileName
import me.danwi.sqlex.parser.util.SqlExMethodExtensionName
import me.danwi.sqlex.parser.util.SqlExSchemaExtensionName
import me.danwi.sqlex.parser.util.isSqlExConfigFileName

//判断这个VirtualFile是否为一个SqlEx配置文件
val VirtualFile?.isSqlExConfig: Boolean
    inline get() = this?.name.isSqlExConfigFileName

//判断这个VirtualFile是否为一个SqlExSchema文件
val VirtualFile?.isSqlExSchema: Boolean
    inline get() = this?.extension == SqlExSchemaExtensionName

//判断这个VirtualFile是否为一个SqlExMethod文件
val VirtualFile?.isSqlExMethod: Boolean
    inline get() = this?.extension == SqlExMethodExtensionName

//读取文件的内容
val VirtualFile.textContent: String?
    inline get() {
        if (this.isDirectory) return null
        return this.inputStream.bufferedReader().use { it.readText() }
    }

//获取文件所在的项目
val VirtualFile.project: Project?
    inline get() = ProjectLocator.getInstance().getProjectsForFile(this).firstOrNull()

//获取文件所在的模块
val VirtualFile.module: Module?
    inline get() {
        val project = this.project ?: return null
        return ProjectRootManager.getInstance(project).fileIndex.getModuleForFile(this)
    }

//获取文件所在的source root
val VirtualFile.sourceRoot: VirtualFile?
    get() {
        val module = this.module ?: return null
        return module.sourceRoots.firstOrNull {
            val thisSegments = this.path.split("/")
            val itSegments = it.path.split("/")
            for ((i, v) in itSegments.withIndex()) if (thisSegments[i] != v) return@firstOrNull false
            true
        }
    }

//获取文件相对于source root的路径
val VirtualFile.sourceRootRelativePath: String?
    inline get() {
        val sourceRoot = this.sourceRoot ?: return null
        return this.path.removePrefix(sourceRoot.path).removePrefix("/")
    }

//获取文件对于project root的路径
val VirtualFile.projectRootRelativePath: String?
    inline get() {
        val projectPath = this.project?.basePath ?: return null
        return this.path.removePrefix(projectPath).removePrefix("/")
    }

//判断文件是否为source root
val VirtualFile.isSourceRoot: Boolean
    inline get() = this == this.sourceRoot

//判断是否为SqlEx源码目录
val VirtualFile.isSqlExSourceRoot: Boolean
    inline get() {
        if (!this.isSourceRoot) return false
        return this.findChild(SqlExConfigFileName) != null
    }

//判断是否在SqlEx源码目录中
val VirtualFile.isInSqlExSourceRoot: Boolean
    inline get() = this.sourceRoot?.isSqlExSourceRoot ?: false

//获取Virtual File对应的SqlExConfig Virtual File
val VirtualFile.configFile: VirtualFile?
    inline get() = this.sourceRoot?.findChild(SqlExConfigFileName)

//获取VirtualFile对应的psi file
val VirtualFile.psiFile: PsiFile?
    inline get() {
        val project = this.project ?: return null
        return PsiManager.getInstance(project).findFile(this)
    }

//获取Virtual File对应的SqlExRepositoryService
val VirtualFile.sqlexRepositoryService: SqlExRepositoryService?
    inline get() {
        if (!this.isInSqlExSourceRoot)
            return null
        return this.module
            ?.sqlexRepositoryServices
            ?.find { it.sourceRoot == this.sourceRoot }
    }

//创建一个虚拟文件
fun createVirtualFile(path: String, content: String): VirtualFile? {
    val file = writeLocalFile(path, content)
    return LocalFileSystem.getInstance().refreshAndFindFileByIoFile(file)
}
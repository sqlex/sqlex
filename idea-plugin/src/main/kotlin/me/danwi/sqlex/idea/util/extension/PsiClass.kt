package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import me.danwi.sqlex.idea.service.sqlexMethodFileCacheKey

//获取全限定包名
val PsiClass.qualifiedPackageName: String?
    inline get() = this.qualifiedName?.removeSuffix(".${this.name}")

//获取该PsiClass对应的SqlEx Method文件
val PsiClass.sqlexMethodFile: VirtualFile?
    inline get() = this.getUserData(sqlexMethodFileCacheKey)
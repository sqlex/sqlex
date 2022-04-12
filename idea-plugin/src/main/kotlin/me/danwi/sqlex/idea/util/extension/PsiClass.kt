package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiClassType
import com.intellij.psi.util.PsiTypesUtil
import me.danwi.sqlex.idea.service.SqlExMethodFileCacheKey
import me.danwi.sqlex.parser.ParameterConverterInterfaceQualifiedName

//获取全限定包名
val PsiClass.qualifiedPackageName: String?
    inline get() = this.qualifiedName?.removeSuffix(".${this.name}")

//获取该PsiClass对应的SqlEx Method文件
val PsiClass.sqlexMethodFile: VirtualFile?
    inline get() = this.getUserData(SqlExMethodFileCacheKey)

//获取PsiClass对应的类型
val PsiClass.classType: PsiClassType
    inline get() = PsiTypesUtil.getClassType(this)

//判断这个psi是否为参数转换器
val PsiClass.isParameterConverter: Boolean
    inline get() = qualifiedName == ParameterConverterInterfaceQualifiedName

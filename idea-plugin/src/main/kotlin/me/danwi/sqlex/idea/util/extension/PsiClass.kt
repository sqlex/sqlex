package me.danwi.sqlex.idea.util.extension

import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiClassType
import com.intellij.psi.util.PsiTypesUtil
import me.danwi.sqlex.common.ParameterTypes.ParameterConverterInterfaceQualifiedName
import me.danwi.sqlex.common.ParameterTypes.PreSupportedTypes
import me.danwi.sqlex.idea.repositroy.SqlExMethodFileCacheKey

//获取全限定包名
val PsiClass.qualifiedPackageName: String?
    inline get() = this.qualifiedName?.removeSuffix(".${this.name}")

//获取该PsiClass对应的SqlEx Method文件
val PsiClass.sqlexMethodFile: VirtualFile?
    inline get() = this.getUserData(SqlExMethodFileCacheKey)

//获取PsiClass对应的类型
val PsiClass.classType: PsiClassType
    inline get() = PsiTypesUtil.getClassType(this)

//判断这个psi是否为参数转换器接口
val PsiClass.isParameterConverterInterface: Boolean
    inline get() = qualifiedName == ParameterConverterInterfaceQualifiedName

//判断这个psi是否为合法的预支持数据类型
val PsiClass?.isPreSupported: Boolean
    get() {
        if (this == null)
            return false
        if (PreSupportedTypes.contains(this.qualifiedName))
            return true
        return this.superClass.isPreSupported
    }

//判断这个psi是否为一个合法的参数转换器
val PsiClass?.isParameterConverter: Boolean
    get() {
        return this
            ?.implementsListTypes
            ?.firstOrNull { it.psiClass?.isParameterConverterInterface == true }
            ?.converterToType
            ?.psiClass
            .isPreSupported
    }

//判断这个psi是否包含无参构造函数
val PsiClass.hasNoArgsConstructor: Boolean
    get() = this.constructors.isEmpty() || this.constructors.any { !it.hasParameters() }
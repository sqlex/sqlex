package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiClassType

//获取psi type的泛型参数
fun PsiClassType.genericArgument(index: Int): PsiClassType? {
    val genericArguments = this.typeArguments().toList()
    if (genericArguments.size <= index)
        return null
    val genericType = genericArguments[index]
    if (genericType !is PsiClassType)
        return null
    return genericType
}

//获取ParameterConverter的来源泛型
val PsiClassType.converterFromType: PsiClassType?
    get() {
        if (this.psiClass?.isParameterConverter == false)
            return null
        return this.genericArgument(0)
    }

//获取ParameterConverter的目的泛型
val PsiClassType.converterToType: PsiClassType?
    get() {
        if (this.psiClass?.isParameterConverter == false)
            return null
        return this.genericArgument(1)
    }

package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.PsiClass
import com.intellij.psi.PsiJavaCodeReferenceElement
import com.intellij.psi.PsiJavaFile
import org.jetbrains.yaml.psi.YAMLFile
import org.jetbrains.yaml.psi.YAMLKeyValue
import org.jetbrains.yaml.psi.YAMLScalar

data class ConverterStore(val fromTypeQualifiedName: String, val psiClass: PsiClass)

//获取配置文件的参数转化器配置
val YAMLFile.converters: List<ConverterStore>?
    get() {
        if (!this.virtualFile.isSqlExConfig)
            return null
        return this
            .childrenOf<YAMLKeyValue>()
            .firstOrNull { it.name == "converters" }
            ?.childrenOf<YAMLScalar>()
            ?.mapNotNull {
                it.injectedOf<PsiJavaFile>()
                    ?.childrenOf<PsiJavaCodeReferenceElement>()
                    ?.firstOrNull()
                    ?.resolve()
            }
            ?.filterIsInstance<PsiClass>()
            ?.mapNotNull {
                val converterInterface =
                    it.implementsListTypes.firstOrNull { item -> item?.psiClass?.isParameterConverterInterface == true }
                if (converterInterface?.converterToType?.psiClass?.isPreSupported != true) {
                    return@mapNotNull null
                }
                val qualifiedName =
                    converterInterface.converterFromType?.psiClass?.qualifiedName ?: return@mapNotNull null
                ConverterStore(qualifiedName, it)
            }
    }

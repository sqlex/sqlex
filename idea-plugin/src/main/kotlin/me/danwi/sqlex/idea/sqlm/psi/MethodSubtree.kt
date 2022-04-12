package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.navigation.ItemPresentation
import com.intellij.psi.PsiMethod
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.service.SqlExMethodPsiClassCacheKey
import me.danwi.sqlex.idea.util.SqlExMethodIcon
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.projectRootRelativePath
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree
import javax.swing.Icon

class MethodSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {
    val returnType: ReturnTypeSubtree?
        get() = children.find { it is ReturnTypeSubtree } as ReturnTypeSubtree?

    val methodName: MethodNameSubtree?
        get() = children.find { it is MethodNameSubtree } as MethodNameSubtree?

    val paramList: ParamListSubtree?
        get() = children.find { it is ParamListSubtree } as ParamListSubtree?

    val sql: SqlSubtree?
        get() = children.find { it is SqlSubtree } as SqlSubtree?

    val javaMethod: PsiMethod?
        get() {
            val javaClass = this.containingFile.virtualFile.getUserData(SqlExMethodPsiClassCacheKey) ?: return null
            val methodName = this.methodName?.methodName ?: return null
            return javaClass.childrenOf<PsiMethod>().find { it.name == methodName }
        }

    override fun getPresentation(): ItemPresentation {
        return MethodItemPresentation(this)
    }
}

class MethodItemPresentation(private val element: MethodSubtree) : ItemPresentation {
    override fun getLocationString(): String {
        return element.containingFile.virtualFile.projectRootRelativePath ?: ""
    }

    override fun getPresentableText(): String {
        val methodName = element.methodName?.methodName ?: "未命名方法"
        val params = element.paramList?.params ?: listOf()
        return "${methodName}(${params.joinToString(", ") { "${it.paramName?.text ?: "_"}:${it.paramType?.text ?: "_"}${if (it.isCollectionParam) "*" else ""}" }})"
    }

    override fun getIcon(unused: Boolean): Icon {
        return SqlExMethodIcon
    }
}
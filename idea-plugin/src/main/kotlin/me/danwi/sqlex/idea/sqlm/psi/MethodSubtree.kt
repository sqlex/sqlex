package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.tree.IElementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class MethodSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {
    val returnType: ReturnTypeSubtree?
        get() = children.find { it is ReturnTypeSubtree } as ReturnTypeSubtree?

    val methodName: MethodNameSubtree?
        get() = children.find { it is MethodNameSubtree } as MethodNameSubtree?

    val paramList: ParamListSubtree?
        get() = children.find { it is ParamListSubtree } as ParamListSubtree?

    val sql: SqlSubtree?
        get() = children.find { it is SqlSubtree } as SqlSubtree?
}
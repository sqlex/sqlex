package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.tree.IElementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class ParamSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {
    val paramName: ParamNameSubtree?
        get() = children.find { it is ParamNameSubtree } as ParamNameSubtree?

    val paramType: ParamTypeSubtree?
        get() = children.find { it is ParamTypeSubtree } as ParamTypeSubtree?

    val isCollectionParam: Boolean
        get() = children.any { it is ParamRepeatSubtree }
}
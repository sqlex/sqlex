package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.tree.IElementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class LBSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {

}

class RBSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {

}
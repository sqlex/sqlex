package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.tree.IElementType
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class RootSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType) {
    val methods: List<MethodSubtree>
        get() {
            return this.children.filterIsInstance<MethodSubtree>()
        }

    val imports: List<ImportSubtree>
        get() {
            return this.children.filterIsInstance<ImportSubtree>()
        }
}
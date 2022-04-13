package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.PsiParserFacade
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.util.extension.sourceRootRelativePath
import me.danwi.sqlex.parser.util.relativePathToPackageName
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

    //添加import语句
    fun addImport(qualifiedName: String) {
        //判断是否已经存在对应类的import语句了
        if (imports.mapNotNull { it.className }.map { it.trim() }.any() { it == qualifiedName.trim() })
            return
        //如果类处于同级包中
        val samePackage = containingFile.virtualFile.sourceRootRelativePath?.relativePathToPackageName ?: return
        if (qualifiedName.startsWith("${samePackage}.") && !qualifiedName.removePrefix("${samePackage}.").contains("."))
            return
        //如果类处于语言包中
        if (qualifiedName.startsWith("java.lang.") && !qualifiedName.removePrefix("java.lang.").contains("."))
            return
        //不存在import语句,新建一个import psi元素
        val importElement = PsiElementFactory.createImport(qualifiedName, project) ?: return
        //最后一个import语句
        val lastImportAnchor = imports.lastOrNull()
        //root下的第一个元素
        val firstChildAnchor = children.firstOrNull()
        //换行元素
        val whiteElement =
            PsiParserFacade.SERVICE.getInstance(project).createWhiteSpaceFromText("\n")
        if (lastImportAnchor != null) {
            //如果最后一个import存在,则添加到它后面
            //先添加一个换行
            addAfter(whiteElement, lastImportAnchor)
            //获取新的换行
            val anchor = imports.lastOrNull()?.nextSibling ?: return
            //添加import语句
            addAfter(importElement, anchor)
        } else if (firstChildAnchor != null) {
            //如果不存在import,则添加到第一个元素前面
            addBefore(importElement, firstChildAnchor)
            val anchor = imports.lastOrNull()
            //添加完成后,加一个换行
            addAfter(whiteElement, anchor)
        } else {
            //空白文件
            add(importElement)
            add(whiteElement)
        }
    }
}
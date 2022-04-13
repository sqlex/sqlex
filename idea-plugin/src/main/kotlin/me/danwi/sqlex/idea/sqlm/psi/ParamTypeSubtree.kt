package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.*
import com.intellij.psi.impl.source.tree.LeafElement
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.idea.util.extension.childrenOf
import me.danwi.sqlex.idea.util.extension.injectedOf
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class ParamTypeSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType),
    PsiLanguageInjectionHost {
    override fun isValidHost(): Boolean {
        return true
    }

    override fun updateText(text: String): PsiLanguageInjectionHost {
        //获取到类名
        val className = text.substringAfterLast(".")
        //获取它第一个自元素,应该是个文本TOKEN
        val textElement = this.firstChild
        if (textElement is LeafElement) {
            //如果能获取到类名,表示text是一个限定名
            if (className.isNotEmpty()) {
                textElement.replaceWithText(className)
                addImport(text)
            } else {
                textElement.replaceWithText(text)
            }
        }
        return this
    }

    override fun createLiteralTextEscaper(): LiteralTextEscaper<out PsiLanguageInjectionHost> {
        return LiteralTextEscaper.createSimple(this)
    }

    //添加import语句
    private fun addImport(qualifiedName: String) {
        //获取对应到virtual file
        val file = this.containingFile
        //必须是一个SqlEx Method文件
        if (file !is SqlExMethodFile)
            return
        //获取root元素
        val root = file.root ?: return
        //判断是否已经存在对应类的import语句了
        if (root.imports.mapNotNull { it.className }.map { it.trim() }.any() { it == qualifiedName.trim() })
            return
        //不存在import语句,新建一个import psi元素
        val importElement = PsiElementFactory.createImport(qualifiedName, project) ?: return
        //最后一个import语句
        val lastImportAnchor = root.imports.lastOrNull()
        //root下的第一个元素
        val firstChildAnchor = root.children.firstOrNull()
        //换行元素
        val whiteElement =
            PsiParserFacade.SERVICE.getInstance(project).createWhiteSpaceFromText("\n")
        if (lastImportAnchor != null) {
            //如果最后一个import存在,则添加到它后面
            //先添加一个换行
            root.addAfter(whiteElement, lastImportAnchor)
            //获取新的换行
            val anchor = root.imports.lastOrNull()?.nextSibling ?: return
            //添加import语句
            root.addAfter(importElement, anchor)
        } else if (firstChildAnchor != null) {
            //如果不存在import,则添加到第一个元素前面
            root.addBefore(importElement, firstChildAnchor)
            val anchor = root.imports.lastOrNull()
            //添加完成后,加一个换行
            root.addAfter(whiteElement, anchor)
        } else {
            //空白文件
            root.add(importElement)
            root.add(whiteElement)
        }
    }

    //获取参数对应的java类型
    val javaType: PsiClass?
        get() {
            val psiClass = this
                .injectedOf<PsiJavaFile>()
                ?.childrenOf<PsiMethod>()
                ?.find { it.name == "dummy" }
                ?.childrenOf<PsiJavaCodeReferenceElement>()
                ?.firstOrNull()
                ?.resolve()
            if (psiClass !is PsiClass) return null
            return psiClass
        }
}
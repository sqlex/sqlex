package me.danwi.sqlex.idea.sqlm.psi

import com.intellij.lang.ASTNode
import com.intellij.psi.*
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.sqlm.SqlExMethodFile
import me.danwi.sqlex.idea.util.extension.sourceRootRelativePath
import me.danwi.sqlex.parser.util.relativePathToPackageName
import org.antlr.intellij.adaptor.psi.IdentifierDefSubtree

class ParamTypeSubtree(node: ASTNode, idElementType: IElementType) : IdentifierDefSubtree(node, idElementType),
    PsiLanguageInjectionHost {
    override fun isValidHost(): Boolean {
        return true
    }

    override fun updateText(text: String): PsiLanguageInjectionHost {
        return ElementManipulators.handleContentChange(this, text)
    }

    override fun createLiteralTextEscaper(): LiteralTextEscaper<out PsiLanguageInjectionHost> {
        return LiteralTextEscaper.createSimple(this)
    }

    //获取参数对应的java类型
    val javaType: PsiClass?
        get() {
            val psiFile = this.containingFile
            if (psiFile !is SqlExMethodFile)
                return null
            //拿到所有的import导入的语句
            val importedClassName = psiFile.root?.imports?.mapNotNull { it.className }
            //本类型的类名
            val className = this.text.trim()
            //查找全限定名
            val qualifiedName = importedClassName?.find { it.endsWith(".${className}") } //import 导入
            val packageName = psiFile.virtualFile.sourceRootRelativePath?.relativePathToPackageName ?: return null
            val qualifiedNameWithSamePackage = "${packageName}.${className}" //同包
            val qualifiedNameWithLanguagePackage = "java.lang.${className}" //语言包
            //获取psi class
            val javaPsiFacade = JavaPsiFacade.getInstance(project)
            //如果全限定名指定了,只从全限定中查找
            if (qualifiedName != null)
                return javaPsiFacade.findClass(qualifiedName, this.resolveScope)
            //如果全限定名中没有,先从同包名下找
            var psiClass = javaPsiFacade.findClass(qualifiedNameWithSamePackage, this.resolveScope)
            //如果同包下没有,则从java.lang中加载查找
            if (psiClass == null)
                psiClass = javaPsiFacade.findClass(qualifiedNameWithLanguagePackage, this.resolveScope)
            //返回psi class
            return psiClass
        }
}
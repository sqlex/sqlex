package me.danwi.sqlex.idea.sqlm

import com.intellij.formatting.*
import com.intellij.lang.ASTNode
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiFile
import com.intellij.psi.TokenType
import com.intellij.psi.codeStyle.CodeStyleSettings
import com.intellij.psi.formatter.common.AbstractBlock
import com.intellij.psi.formatter.common.InjectedLanguageBlockBuilder
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_IMPORT
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_METHOD
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_METHOD_NAME
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_PARAM
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_PARAM_LIST
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_PARAM_NAME
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_PARAM_REPEAT
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_PARAM_TYPE
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_RETURN_TYPE
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.RULE_SQL
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_COLON
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_COMMA
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LCB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RCB

class SqlExMethodBlock(
    node: ASTNode,
    wrap: Wrap?,
    alignment: Alignment?,
    private val settings: CodeStyleSettings,
    private val spacingBuilder: SpacingBuilder
) : AbstractBlock(node, wrap, alignment) {
    override fun isLeaf(): Boolean {
        return myNode.firstChildNode == null
    }

    override fun getSpacing(child1: Block?, child2: Block): Spacing? {
        return spacingBuilder.getSpacing(this, child1, child2)
    }

    override fun buildChildren(): MutableList<Block> {
        val blocks = mutableListOf<Block>()
        var child = myNode.firstChildNode
        while (child != null) {
            if (child.elementType == RULE_SQL) {
                blocks.add(
                    SqlExMethodSqlBlock(
                        child,
                        Wrap.createWrap(WrapType.NONE, false),
                        Alignment.createAlignment(),
                        settings,
                        spacingBuilder
                    )
                )
            } else if (child.elementType != TokenType.WHITE_SPACE) {
                blocks.add(
                    SqlExMethodBlock(
                        child,
                        Wrap.createWrap(WrapType.NONE, false),
                        Alignment.createAlignment(),
                        settings,
                        spacingBuilder
                    )
                )
            }
            child = child.treeNext
        }
        return blocks
    }

    override fun getIndent(): Indent? {
        return Indent.getNoneIndent()
    }
}

class SqlExMethodSqlBlock(
    node: ASTNode,
    wrap: Wrap?,
    alignment: Alignment?,
    settings: CodeStyleSettings,
    private val spacingBuilder: SpacingBuilder
) : AbstractBlock(node, wrap, alignment) {

    private val sqlExInjectedLanguageBlockBuilder = SqlExInjectedLanguageBlockBuilder(settings)

    override fun isLeaf(): Boolean {
        return myNode.firstChildNode == null
    }

    override fun getSpacing(child1: Block?, child2: Block): Spacing? {
        return spacingBuilder.getSpacing(this, child1, child2)
    }

    override fun buildChildren(): MutableList<Block> {
        val result = mutableListOf<Block>()
        sqlExInjectedLanguageBlockBuilder.addInjectedBlocks(
            result,
            this.node,
            Wrap.createWrap(WrapType.NONE, false),
            Alignment.createAlignment(),
            Indent.getNoneIndent()
        )
        return result
    }

    override fun getIndent(): Indent? {
        return Indent.getSpaceIndent(4)
    }
}

class SqlExInjectedLanguageBlockBuilder(private val settings: CodeStyleSettings) : InjectedLanguageBlockBuilder() {
    override fun getSettings(): CodeStyleSettings {
        return settings
    }

    override fun canProcessFragment(text: String?, injectionHost: ASTNode?): Boolean {
        return true
    }

    override fun createBlockBeforeInjection(
        node: ASTNode?, wrap: Wrap?, alignment: Alignment?, indent: Indent?, range: TextRange?
    ): Block {
        return SqlExMethodBlock(node!!, wrap, alignment, settings, createSpaceBuilder(settings))
    }

    override fun createBlockAfterInjection(
        node: ASTNode?, wrap: Wrap?, alignment: Alignment?, indent: Indent?, range: TextRange?
    ): Block {
        return SqlExMethodBlock(node!!, wrap, alignment, settings, createSpaceBuilder(settings))
    }
}

fun createSpaceBuilder(settings: CodeStyleSettings): SpacingBuilder {
    return SpacingBuilder(settings, SqlExMethodLanguage.INSTANCE)
        //import语句换行
        .between(RULE_IMPORT, RULE_IMPORT).blankLines(0)
        //import语句和方法之间必须有换行
        .between(RULE_IMPORT, RULE_METHOD).blankLines(1)
        //方法之间必须换行
        .between(RULE_METHOD, RULE_METHOD).blankLines(1)
        //返回类型1方法名
        .between(RULE_RETURN_TYPE, RULE_METHOD_NAME).spaces(1)
        //方法名0(
        .between(RULE_METHOD_NAME, TOKEN_LB).none()
        //参数名0:0参数类型
        .between(RULE_PARAM_NAME, TOKEN_COLON).none()
        .between(TOKEN_COLON, RULE_PARAM_TYPE).none()
        //参数类型0*
        .between(RULE_PARAM_TYPE, RULE_PARAM_REPEAT).none()
        //参数, 参数
        .between(RULE_PARAM, TOKEN_COMMA).none()
        .between(TOKEN_COMMA, RULE_PARAM).spaces(1)
        //(0参数列表0)
        .between(TOKEN_LB, RULE_PARAM_LIST).none()
        .between(RULE_PARAM_LIST, TOKEN_RB).none()
        //)1{
        .between(TOKEN_RB, TOKEN_LCB).spaces(1)
        //sql必须另起一行
        .before(RULE_SQL).blankLines(0).after(RULE_SQL).none()
        //0}0
        .before(TOKEN_RCB).none().after(TOKEN_RCB).none()
}

class SqlExMethodFormattingModelBuilder : FormattingModelBuilder {
    override fun createModel(formattingContext: FormattingContext): FormattingModel {
        return FormattingModelProvider.createFormattingModelForPsiFile(
            formattingContext.containingFile, SqlExMethodBlock(
                formattingContext.node,
                Wrap.createWrap(WrapType.NONE, false),
                Alignment.createAlignment(),
                formattingContext.codeStyleSettings,
                createSpaceBuilder(formattingContext.codeStyleSettings)
            ), formattingContext.codeStyleSettings
        )
    }

    override fun getRangeAffectingIndent(file: PsiFile?, offset: Int, elementAtOffset: ASTNode?): TextRange? {
        return null
    }
}
package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.ASTNode
import com.intellij.lang.ParserDefinition
import com.intellij.lang.PsiParser
import com.intellij.lexer.Lexer
import com.intellij.openapi.project.Project
import com.intellij.psi.FileViewProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IElementType
import com.intellij.psi.tree.IFileElementType
import com.intellij.psi.tree.TokenSet
import me.danwi.sqlex.idea.sqlm.psi.*
import me.danwi.sqlex.parser.SqlExMethodLanguageLexer
import me.danwi.sqlex.parser.SqlExMethodLanguageParser
import org.antlr.intellij.adaptor.lexer.ANTLRLexerAdaptor
import org.antlr.intellij.adaptor.lexer.PSIElementTypeFactory
import org.antlr.intellij.adaptor.lexer.RuleIElementType
import org.antlr.intellij.adaptor.lexer.TokenIElementType
import org.antlr.intellij.adaptor.parser.ANTLRParserAdaptor
import org.antlr.intellij.adaptor.psi.ANTLRPsiNode
import org.antlr.v4.runtime.Parser
import org.antlr.v4.runtime.tree.ParseTree

open class SqlExMethodParserDefinition : ParserDefinition {
    companion object {
        init {
            PSIElementTypeFactory.defineLanguageIElementTypes(
                SqlExMethodLanguage.INSTANCE,
                SqlExMethodLanguageParser.tokenNames,
                SqlExMethodLanguageParser.ruleNames
            )
        }

        private val tokens = PSIElementTypeFactory.getTokenIElementTypes(SqlExMethodLanguage.INSTANCE)
        private val rules = PSIElementTypeFactory.getRuleIElementTypes(SqlExMethodLanguage.INSTANCE)

        val RULE_ROOT: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_root]
        val RULE_IMPORT: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_importEx]
        val RULE_CLASS_NAME: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_className]
        val RULE_METHOD: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_method]
        val RULE_RETURN_TYPE: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_returnType]
        val RULE_METHOD_NAME: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_methodName]
        val RULE_PARAM_LIST: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_paramList]
        val RULE_PARAM: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_param]
        val RULE_PARAM_NAME: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_paramName]
        val RULE_PARAM_TYPE: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_paramType]
        val RULE_PARAM_REPEAT: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_paramRepeat]
        val RULE_SQL: RuleIElementType = rules[SqlExMethodLanguageParser.RULE_sql]

        val TOKEN_IMPORT: TokenIElementType = tokens[SqlExMethodLanguageLexer.IMPORT]
        val TOKEN_WS: TokenIElementType = tokens[SqlExMethodLanguageLexer.WS]
        val TOKEN_SQL_WS: TokenIElementType = tokens[SqlExMethodLanguageLexer.SQL_WS]
        val TOKEN_SQL: TokenIElementType = tokens[SqlExMethodLanguageLexer.SQL]
        val TOKEN_SQL_TEXT: TokenIElementType = tokens[SqlExMethodLanguageLexer.SQL_TEXT]
        val TOKEN_COMMENT: TokenIElementType = tokens[SqlExMethodLanguageLexer.COMMENT]
        val TOKEN_ID: TokenIElementType = tokens[SqlExMethodLanguageLexer.ID]
        val TOKEN_LB: TokenIElementType = tokens[SqlExMethodLanguageLexer.LB]
        val TOKEN_RB: TokenIElementType = tokens[SqlExMethodLanguageLexer.RB]
        val TOKEN_LCB: TokenIElementType = tokens[SqlExMethodLanguageLexer.LCB]
        val TOKEN_RCB: TokenIElementType = tokens[SqlExMethodLanguageLexer.RCB]
        val TOKEN_COLON: TokenIElementType = tokens[SqlExMethodLanguageLexer.COLON]
        val TOKEN_COMMA: TokenIElementType = tokens[SqlExMethodLanguageLexer.COMMA]
        val TOKEN_ERR_CHAR: TokenIElementType = tokens[SqlExMethodLanguageLexer.ERRCHAR]


        val FILE = IFileElementType(SqlExMethodLanguage.INSTANCE)

        val WHITESPACE: TokenSet = TokenSet.create(TOKEN_WS, TOKEN_SQL_WS, TOKEN_ERR_CHAR)
        val COMMENTS: TokenSet = TokenSet.create(TOKEN_COMMENT)
        val STRING: TokenSet = TokenSet.EMPTY
    }

    override fun getFileNodeType(): IFileElementType {
        return FILE
    }

    override fun getWhitespaceTokens(): TokenSet {
        return WHITESPACE
    }

    override fun getCommentTokens(): TokenSet {
        return COMMENTS
    }

    override fun getStringLiteralElements(): TokenSet {
        return STRING
    }

    override fun createElement(node: ASTNode): PsiElement {
        if (node.elementType is TokenIElementType) {
            return ANTLRPsiNode(node)
        }
        if (node.elementType !is RuleIElementType) {
            return ANTLRPsiNode(node)
        }
        return when (node.elementType) {
            RULE_ROOT -> RootSubtree(node, node.elementType)
            RULE_IMPORT -> ImportSubtree(node, node.elementType)
            RULE_CLASS_NAME -> ClassNameSubtree(node, node.elementType)
            RULE_METHOD -> MethodSubtree(node, node.elementType)
            RULE_RETURN_TYPE -> ReturnTypeSubtree(node, node.elementType)
            RULE_METHOD_NAME -> MethodNameSubtree(node, node.elementType)
            RULE_PARAM_LIST -> ParamListSubtree(node, node.elementType)
            RULE_PARAM -> ParamSubtree(node, node.elementType)
            RULE_PARAM_NAME -> ParamNameSubtree(node, node.elementType)
            RULE_PARAM_TYPE -> ParamTypeSubtree(node, node.elementType)
            RULE_PARAM_REPEAT -> ParamRepeatSubtree(node, node.elementType)
            RULE_SQL -> SqlSubtree(node, node.elementType)
            else -> ANTLRPsiNode(node)
        }
    }

    override fun createFile(viewProvider: FileViewProvider): PsiFile {
        return SqlExMethodFile(viewProvider)
    }

    override fun createLexer(p0: Project?): Lexer {
        return ANTLRLexerAdaptor(SqlExMethodLanguage.INSTANCE, SqlExMethodLanguageLexer(null))
    }

    override fun createParser(p0: Project?): PsiParser {
        return object : ANTLRParserAdaptor(SqlExMethodLanguage.INSTANCE, SqlExMethodLanguageParser(null)) {
            override fun parse(parser: Parser, root: IElementType): ParseTree {
                return (parser as SqlExMethodLanguageParser).root()
            }
        }
    }
}
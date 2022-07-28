package me.danwi.sqlex.idea.sqlm

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.editor.colors.TextAttributesKey.createTextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_COMMENT
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_ID
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_IMPORT
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LCB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RCB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_STAR
import me.danwi.sqlex.parser.SqlExMethodLanguageLexer
import org.antlr.intellij.adaptor.lexer.ANTLRLexerAdaptor

class SqlExMethodSyntaxHighlighter : SyntaxHighlighterBase() {
    companion object {
        private val BRACES = arrayOf(createTextAttributesKey("SQLEX_BRACES", DefaultLanguageHighlighterColors.BRACES))
        private val KEYWORD =
            arrayOf(createTextAttributesKey("SQLEX_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD))
        private val STAR =
            arrayOf(createTextAttributesKey("SQLEX_STAR", DefaultLanguageHighlighterColors.OPERATION_SIGN))
        private val METHOD =
            arrayOf(createTextAttributesKey("SQLEX_METHOD", DefaultLanguageHighlighterColors.INSTANCE_METHOD))
        private val COMMENT =
            arrayOf(createTextAttributesKey("SQLEX_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT))
    }

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> {
        return when (tokenType) {
            TOKEN_IMPORT -> KEYWORD
            TOKEN_ID -> METHOD
            TOKEN_COMMENT -> COMMENT
            TOKEN_LB, TOKEN_RB, TOKEN_LCB, TOKEN_RCB -> BRACES
            TOKEN_STAR -> STAR
            else -> TextAttributesKey.EMPTY_ARRAY
        }
    }

    override fun getHighlightingLexer(): Lexer {
        return ANTLRLexerAdaptor(SqlExMethodLanguage.INSTANCE, SqlExMethodLanguageLexer(null))
    }
}
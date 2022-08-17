package me.danwi.sqlex.idea.sqlm

import com.intellij.lang.BracePair
import com.intellij.lang.PairedBraceMatcher
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IElementType
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_LCB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RB
import me.danwi.sqlex.idea.sqlm.SqlExMethodParserDefinition.Companion.TOKEN_RCB

class SqlExMethodBraceMatcher : PairedBraceMatcher {
    override fun getCodeConstructStart(file: PsiFile?, openingBraceOffset: Int): Int {
        return openingBraceOffset
    }

    override fun getPairs(): Array<BracePair> {
        return arrayOf(
            BracePair(TOKEN_LB, TOKEN_RB, true),
            BracePair(TOKEN_LCB, TOKEN_RCB, true),
        )
    }

    override fun isPairedBracesAllowedBeforeType(lbraceType: IElementType, contextType: IElementType?): Boolean {
        return true
    }
}

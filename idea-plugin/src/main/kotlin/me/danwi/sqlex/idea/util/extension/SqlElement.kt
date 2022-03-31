package me.danwi.sqlex.idea.util.extension

import com.intellij.psi.util.PsiTreeUtil
import com.intellij.sql.psi.SqlElement
import me.danwi.sqlex.idea.sqlm.psi.MethodSubtree
import me.danwi.sqlex.idea.sqlm.psi.SqlSubtree

//SQL元素是否包含在一个SqlEx Method的方法中,如果是,就返回这个方法的psi元素
val SqlElement.containingSqlExMethod: MethodSubtree?
    inline get() {
        val sqlSubtree = PsiTreeUtil.getContextOfType(this, SqlSubtree::class.java) ?: return null
        return sqlSubtree.parent as MethodSubtree?
    }
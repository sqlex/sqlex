package me.danwi.sqlex.idea.sqlm.spring

import com.intellij.psi.PsiArrayInitializerMemberValue
import com.intellij.psi.PsiClass
import com.intellij.psi.PsiClassObjectAccessExpression
import com.intellij.spring.contexts.model.LocalModel
import com.intellij.spring.model.CommonSpringBean
import com.intellij.spring.model.custom.CustomLocalComponentsDiscoverer
import com.intellij.spring.model.jam.stereotype.CustomSpringComponent
import me.danwi.sqlex.core.annotation.repository.SqlExMethods
import me.danwi.sqlex.idea.util.extension.psiClass
import me.danwi.sqlex.spring.ImportSqlEx
import java.util.*

class SqlExMethodComponentDiscoverer : CustomLocalComponentsDiscoverer() {
    override fun getCustomComponents(model: LocalModel<*>): MutableCollection<CommonSpringBean> {
        //获取其配置类
        val configurationClass = model.config
        if (configurationClass !is PsiClass)
            return Collections.emptyList()
        //获取到他所有到ImportSqlEx注解,并提取其中的Repository值
        val repositoryClasses = configurationClass.annotations
            .filter { it.qualifiedName == ImportSqlEx::class.java.name } //ImportSqlEx注解
            .mapNotNull { it.findAttributeValue("value") } //拿到所有的Class值
            .filterIsInstance<PsiClassObjectAccessExpression>()
            .mapNotNull { it.operand.type.psiClass }
        //通过SqlExMethods注解,找到挂在其上的所有SqlExMethod类
        val daoClasses = repositoryClasses
            .mapNotNull { it.getAnnotation(SqlExMethods::class.java.name)?.findAttributeValue("value") }
            .flatMap {
                when (it) {
                    //单个类例如@SqlExMethods(XxxDao.class)
                    is PsiClassObjectAccessExpression -> return@flatMap listOf(it)
                    //多个类例如@SqlExMethods({XxxDao.class,XxxDao2.class})
                    is PsiArrayInitializerMemberValue -> return@flatMap it.initializers.toList()
                    else -> return@flatMap listOf()
                }
            }
            .filterIsInstance<PsiClassObjectAccessExpression>()
            .mapNotNull { it.operand.type.psiClass }
        //构造自定义的Spring组件,返回
        return daoClasses.map { CustomSpringComponent(it) }.toMutableList()
    }
}
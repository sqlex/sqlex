package me.danwi.sqlex.idea.sqlm.spring

import com.intellij.psi.search.FileTypeIndex
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.spring.contexts.model.LocalModel
import com.intellij.spring.model.CommonSpringBean
import com.intellij.spring.model.custom.CustomLocalComponentsDiscoverer
import com.intellij.spring.model.jam.stereotype.CustomSpringComponent
import me.danwi.sqlex.idea.service.SqlExMethodPsiClassCacheKey
import me.danwi.sqlex.idea.sqlm.SqlExMethodFileType
import java.util.Collections

class SqlExMethodComponentDiscoverer : CustomLocalComponentsDiscoverer() {
    override fun getCustomComponents(model: LocalModel<*>): MutableCollection<CommonSpringBean> {
        val module = model.module ?: return Collections.emptyList()
        //TODO: 暂时不处理ImportSqlEx注解信息,所有的Dao接口默认导入
        return FileTypeIndex
            .getFiles(
                SqlExMethodFileType.INSTANCE,
                GlobalSearchScope.moduleWithDependenciesAndLibrariesScope(module)
            )
            .mapNotNull { it.getUserData(SqlExMethodPsiClassCacheKey) }
            .map { CustomSpringComponent(it) }
            .toMutableList()
    }
}
package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.parser.Repository
import me.danwi.sqlex.parser.util.pascalName

class GeneratedEntityFile(
    rootPackage: String,
    private val tableName: String,
    private val repository: Repository,
) : GeneratedJavaFile(rootPackage, tableName.pascalName) {
    override fun generate(): TypeSpec {
        val tableInfo = repository.getTableInfo(tableName)
        return tableInfo.columns.toEntityClass(tableName.pascalName, isStatic = false, nullableAnnotation = true)
    }
}
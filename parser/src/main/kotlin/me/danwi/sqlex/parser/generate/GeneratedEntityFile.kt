package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.parser.Session
import me.danwi.sqlex.parser.util.pascalName

class GeneratedEntityFile(
    rootPackage: String,
    private val tableName: String,
    private val session: Session,
) : GeneratedJavaFile(rootPackage, tableName.pascalName) {
    override fun generate(): TypeSpec {
        val columns = session.getColumns(tableName)
        return columns.toEntityClass(tableName.pascalName)
    }
}
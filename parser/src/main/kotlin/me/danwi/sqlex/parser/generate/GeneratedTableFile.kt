package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.core.ExceptionTranslator
import me.danwi.sqlex.core.annotation.SqlExRepository
import me.danwi.sqlex.core.jdbc.ParameterSetter
import me.danwi.sqlex.core.query.TableDelete
import me.danwi.sqlex.core.query.TableInsert
import me.danwi.sqlex.core.query.TableQuery
import me.danwi.sqlex.core.query.TableUpdate
import me.danwi.sqlex.core.query.Column
import me.danwi.sqlex.core.transaction.TransactionManager
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.Session
import me.danwi.sqlex.parser.util.pascalName
import javax.lang.model.element.Modifier

class GeneratedTableFile(
    private val rootPackage: String,
    private val tableName: String,
    entityQualifiedName: String,
    private val session: Session
) : GeneratedJavaFile(rootPackage, "${tableName.pascalName}Table") {
    //实体类
    private val entityTypeName: ClassName = ClassName.bestGuess(DoNotHidePackagePrefix + entityQualifiedName)

    //更新类
    private val updateClassName = "Update"
    private val updateClassTypeName = ClassName.get(rootPackage, className, updateClassName)

    override fun generate(): TypeSpec {
        val typeSpecBuilder = TypeSpec.classBuilder(className).addModifiers(Modifier.PUBLIC)
            .superclass(ParameterizedTypeName.get(ClassName.get(TableInsert::class.java), entityTypeName))
            .addAnnotation(
                //所属的repository
                AnnotationSpec
                    .builder(SqlExRepository::class.java)
                    .addMember("value", "\$T.class", ClassName.get(rootPackage, RepositoryClassName))
                    .build()
            )

        //添加字段
        typeSpecBuilder.addFields(generateFields())
        //添加构造函数
        typeSpecBuilder.addMethod(generateConstructorMethod())

        val columns = session.getColumns(tableName)
        //添加静态列表达式字段
        typeSpecBuilder.addFields(generateColumnExpression(columns))
        //添加update类
        typeSpecBuilder.addType(generateUpdateClass(columns))
        //添加update方法
        typeSpecBuilder.addMethod(generateUpdateMethod())
        //添加delete方法
        typeSpecBuilder.addMethod(generateDeleteMethod())
        //添加select方法
        typeSpecBuilder.addMethod(generateSelectMethod())

        return typeSpecBuilder.build()
    }

    private fun generateColumnExpression(columns: Array<Field>): List<FieldSpec> {
        return columns.map {
            FieldSpec.builder(ClassName.get(Column::class.java), it.name.pascalName)
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
                .initializer(
                    CodeBlock.of(
                        "new \$T(\$S,\$S, \$S,\$L,\$L, \$L,\$L,\$L, \$L,\$L,\$L, \$L,\$L)",
                        ClassName.get(Column::class.java),
                        tableName, it.name,
                        it.dbType, "java.sql.JDBCType.${it.JdbcType.name}", it.length,
                        it.unsigned, it.binary, it.decimal,
                        it.isPrimaryKey, it.isAutoIncrement, it.isUnique,
                        it.notNull, it.hasDefaultVale
                    )
                )
                .build()
        }
    }

    private fun generateFields(): List<FieldSpec> {
        return listOf(
            FieldSpec.builder(
                ClassName.get(TransactionManager::class.java),
                "transactionManager",
                Modifier.PRIVATE,
                Modifier.FINAL
            ).build(),
            FieldSpec.builder(
                ClassName.get(ParameterSetter::class.java),
                "parameterSetter",
                Modifier.PRIVATE,
                Modifier.FINAL
            ).build(),
            FieldSpec.builder(
                ClassName.get(ExceptionTranslator::class.java),
                "translator",
                Modifier.PRIVATE,
                Modifier.FINAL
            ).build()
        )
    }

    private fun generateConstructorMethod(): MethodSpec {
        return MethodSpec.constructorBuilder()
            .addModifiers(Modifier.PUBLIC)
            .addParameter(ClassName.get(TransactionManager::class.java), "transactionManager")
            .addParameter(ClassName.get(ParameterSetter::class.java), "parameterSetter")
            .addParameter(ClassName.get(ExceptionTranslator::class.java), "translator")
            .addCode("super(\$S, transactionManager, parameterSetter, translator);\n", tableName)
            .addCode("this.transactionManager = transactionManager;\n")
            .addCode("this.parameterSetter = parameterSetter;\n")
            .addCode("this.translator = translator;")
            .build()
    }

    private fun generateUpdateClass(columns: Array<Field>): TypeSpec {
        val typeSpecBuilder = TypeSpec.classBuilder(updateClassName)
            .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
            .superclass(
                ParameterizedTypeName.get(
                    ClassName.get(TableUpdate::class.java),
                    updateClassTypeName
                )
            )
        //添加构造函数
        typeSpecBuilder.addMethod(
            MethodSpec.constructorBuilder()
                .addModifiers(Modifier.PUBLIC)
                .addParameter(ClassName.get(TransactionManager::class.java), "transactionManager")
                .addParameter(ClassName.get(ParameterSetter::class.java), "parameterSetter")
                .addParameter(ClassName.get(ExceptionTranslator::class.java), "translator")
                .addCode("super(\$S, transactionManager, parameterSetter, translator);\n", tableName)
                .build()
        )
        //添加set方法
        typeSpecBuilder.addMethods(
            columns.map {
                MethodSpec.methodBuilder("set${it.name.pascalName}")
                    .addModifiers(Modifier.PUBLIC)
                    .returns(updateClassTypeName)
                    .addParameter(it.JavaType, "value")
                    .addCode("super.values.put(\$S, value);\n", it.name)
                    .addCode("return this;")
                    .build()
            }
        )

        return typeSpecBuilder.build()
    }

    private fun generateUpdateMethod(): MethodSpec {
        return MethodSpec.methodBuilder("update")
            .addModifiers(Modifier.PUBLIC)
            .returns(updateClassTypeName)
            .addCode(
                "return new \$T(this.transactionManager, this.parameterSetter, this.translator);",
                updateClassTypeName
            )
            .build()
    }

    private fun generateDeleteMethod(): MethodSpec {
        return MethodSpec.methodBuilder("delete")
            .addModifiers(Modifier.PUBLIC)
            .returns(ClassName.get(TableDelete::class.java))
            .addCode(
                "return new \$T(\$S, this.transactionManager, this.parameterSetter, this.translator);",
                TableDelete::class.java, tableName
            )
            .build()
    }

    private fun generateSelectMethod(): MethodSpec {
        return MethodSpec.methodBuilder("select")
            .addModifiers(Modifier.PUBLIC)
            .returns(
                ParameterizedTypeName.get(
                    ClassName.get(TableQuery::class.java),
                    entityTypeName
                )
            )
            .addCode(
                "return new \$T<>(\$S, this.transactionManager, this.parameterSetter, this.translator, \$T.class);",
                TableQuery::class.java, tableName, entityTypeName
            )
            .build()
    }
}
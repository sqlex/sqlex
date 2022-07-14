package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.ClassName
import com.squareup.javapoet.CodeBlock
import com.squareup.javapoet.FieldSpec
import com.squareup.javapoet.MethodSpec
import com.squareup.javapoet.ParameterizedTypeName
import com.squareup.javapoet.TypeSpec
import me.danwi.sqlex.core.query.TableDelete
import me.danwi.sqlex.core.query.TableInsert
import me.danwi.sqlex.core.query.TableQuery
import me.danwi.sqlex.core.query.TableUpdate
import me.danwi.sqlex.core.query.expression.ColumnExpression
import me.danwi.sqlex.core.transaction.TransactionManager
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.Session
import me.danwi.sqlex.parser.util.pascalName
import javax.lang.model.element.Modifier

class GeneratedTableFile(
    rootPackage: String,
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

        //添加字段
        typeSpecBuilder.addField(
            ClassName.get(TransactionManager::class.java),
            "transactionManager",
            Modifier.PRIVATE,
            Modifier.FINAL
        )
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
            FieldSpec.builder(ClassName.get(ColumnExpression::class.java), it.name.pascalName)
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
                .initializer(
                    CodeBlock.of(
                        "new \$T(\$S,\$S)",
                        ClassName.get(ColumnExpression::class.java),
                        tableName, it.name
                    )
                )
                .build()
        }
    }

    private fun generateConstructorMethod(): MethodSpec {
        return MethodSpec.constructorBuilder()
            .addModifiers(Modifier.PUBLIC)
            .addParameter(ClassName.get(TransactionManager::class.java), "transactionManager")
            .addCode("super(transactionManager, \$T.class);\n", entityTypeName)
            .addCode("this.transactionManager = transactionManager;")
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
                .addCode("super(transactionManager, \$T.class);\n", entityTypeName)
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
            .addCode("return new \$T(this.transactionManager);", updateClassTypeName)
            .build()
    }

    private fun generateDeleteMethod(): MethodSpec {
        return MethodSpec.methodBuilder("delete")
            .addModifiers(Modifier.PUBLIC)
            .returns(ClassName.get(TableDelete::class.java))
            .addCode("return new \$T(this.transactionManager);", TableDelete::class.java)
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
            .addCode("return new \$T<>(this.transactionManager, \$T.class);", TableQuery::class.java, entityTypeName)
            .build()
    }
}
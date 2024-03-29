package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.core.annotation.SqlExRepository
import me.danwi.sqlex.core.annotation.SqlExTableAccessObject
import me.danwi.sqlex.core.jdbc.RawSQLExecutor
import me.danwi.sqlex.core.query.*
import me.danwi.sqlex.core.query.expression.Expression
import me.danwi.sqlex.parser.Field
import me.danwi.sqlex.parser.Repository
import me.danwi.sqlex.parser.TableInfo
import me.danwi.sqlex.parser.util.pascalName
import org.jetbrains.annotations.NotNull
import org.jetbrains.annotations.Nullable
import javax.lang.model.element.Modifier

class GeneratedTableFile(
    private val rootPackage: String,
    private val tableName: String,
    entityQualifiedName: String,
    private val repository: Repository
) : GeneratedJavaFile(rootPackage, "${tableName.pascalName}Table") {
    //实体类
    private val entityTypeName: ClassName = ClassName.bestGuess(DoNotHidePackagePrefix + entityQualifiedName)

    //更新类
    private val updateClassName = "Update"
    private val updateClassTypeName = ClassName.get(rootPackage, className, updateClassName)

    override fun generate(): TypeSpec {
        //获取表信息
        val tableInfo = repository.getTableInfo(tableName)

        //获取生成列
        val generatedColumn = tableInfo.columns.find { it.isAutoIncrement }
        val generatedColumnJavaType = generatedColumn?.JavaType ?: ClassName.get(Void::class.java)

        //构建表操作类
        val typeSpecBuilder = TypeSpec.classBuilder(className).addModifiers(Modifier.PUBLIC)
            //继承至 TableInsert<实体,生成列>
            .superclass(
                ParameterizedTypeName.get(
                    ClassName.get(TableInsert::class.java),
                    entityTypeName,
                    generatedColumnJavaType
                )
            )
            .addAnnotation(
                //所属的repository
                AnnotationSpec
                    .builder(SqlExRepository::class.java)
                    .addMember("value", "\$T.class", ClassName.get(rootPackage, RepositoryClassName))
                    .build()
            )
            .addAnnotation(SqlExTableAccessObject::class.java)

        //添加字段
        typeSpecBuilder.addFields(generateFields())
        //添加构造函数
        typeSpecBuilder.addMethod(generateConstructorMethod(generatedColumn))
        //添加静态列表达式字段
        typeSpecBuilder.addFields(generateColumnExpression(tableInfo.columns))
        //添加update类
        typeSpecBuilder.addType(generateUpdateClass(tableInfo.columns))
        //添加update方法
        typeSpecBuilder.addMethod(generateUpdateMethod())
        //添加delete方法
        typeSpecBuilder.addMethod(generateDeleteMethod())
        //添加select方法
        typeSpecBuilder.addMethod(generateSelectMethod())
        //添加短链接方法
        typeSpecBuilder.addMethods(generateShortCutMethods(tableInfo))

        return typeSpecBuilder.build()
    }

    private fun generateColumnExpression(columns: Array<Field>): List<FieldSpec> {
        return columns.map {
            FieldSpec.builder(ClassName.get(Column::class.java), it.name.pascalName)
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC, Modifier.FINAL)
                .initializer(
                    CodeBlock.of(
                        "new \$T(\$S,\$S, \$S,\$L,\$LL, \$L,\$L,\$LL, \$L,\$L,\$L, \$L,\$L)",
                        ClassName.get(Column::class.java),
                        tableName, it.name,
                        it.dbType, "java.sql.JDBCType.${it.JdbcType.name}", it.length,
                        it.unsigned, it.binary, it.decimal,
                        it.isPrimaryKey, it.isAutoIncrement, it.isUnique,
                        it.notNull, it.hasDefaultValue
                    )
                )
                .build()
        }
    }

    private fun generateFields(): List<FieldSpec> {
        return listOf(
            FieldSpec.builder(
                RawSQLExecutor::class.java,
                "executor",
                Modifier.PRIVATE,
                Modifier.FINAL
            ).build(),
        )
    }

    private fun generateConstructorMethod(generatedColumn: Field?): MethodSpec {
        val constructorMethod = MethodSpec.constructorBuilder()
            .addModifiers(Modifier.PUBLIC)
            .addParameter(RawSQLExecutor::class.java, "executor")
        //如有生成列,则把生成列的类型传给TableInsert构造函数,没有则传空
        if (generatedColumn != null)
            constructorMethod.addCode(
                "super(\$S, executor, \$T.class);\n",
                tableName, generatedColumn.JavaType
            )
        else
            constructorMethod.addCode("super(\$S, executor, null);\n", tableName)

        return constructorMethod
            .addCode("this.executor = executor;")
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
                .addParameter(RawSQLExecutor::class.java, "executor")
                .addCode("super(\$S, executor);\n", tableName)
                .build()
        )
        //添加set方法
        typeSpecBuilder.addMethods(
            columns.map {
                val parameterSpec = ParameterSpec.builder(it.JavaType, "value")
                    .addAnnotation(if (it.notNull) NotNull::class.java else Nullable::class.java)
                MethodSpec.methodBuilder("set${it.name.pascalName}")
                    .addModifiers(Modifier.PUBLIC)
                    .returns(updateClassTypeName)
                    .addParameter(parameterSpec.build())
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
                "return new \$T(this.executor);",
                updateClassTypeName
            )
            .build()
    }

    private fun generateDeleteMethod(): MethodSpec {
        return MethodSpec.methodBuilder("delete")
            .addModifiers(Modifier.PUBLIC)
            .returns(ClassName.get(TableDelete::class.java))
            .addCode(
                "return new \$T(\$S, this.executor);",
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
                "return new \$T<>(\$S, this.executor, \$T.class);",
                TableQuery::class.java, tableName, entityTypeName
            )
            .build()
    }

    private fun generateShortCutMethods(tableInfo: TableInfo): List<MethodSpec> {
        //获取主键/唯一列,如果列信息不匹配(不可能出现),则返回空
        val uniqueColumns =
            tableInfo.uniques.map { it.map { tableInfo.columns.find { c -> c.name == it } ?: return listOf() } }
        //查找方法
        val findMethods = uniqueColumns
            .mapNotNull {
                val parameters = it.map { c -> ParameterSpec.builder(c.JavaType, c.name.pascalName).build() }
                val whereCodeSegments = it.map { c ->
                    CodeBlock.of(
                        "where(${className}.${c.name.pascalName}.eq(\$T.arg(${c.name.pascalName})))",
                        Expression::class.java
                    )
                }
                MethodSpec.methodBuilder("findBy${it.joinToString("And") { c -> c.name.pascalName }}")
                    .addAnnotation(Nullable::class.java)
                    .addModifiers(Modifier.PUBLIC)
                    .returns(entityTypeName)
                    .addParameters(parameters)
                    .addCode("return this.select().")
                    .addCode(CodeBlock.join(whereCodeSegments, "."))
                    .addCode(".findOne();")
                    .build()

            }
        //删除方法
        val deleteMethods = uniqueColumns
            .mapNotNull {
                val parameters = it.map { c -> ParameterSpec.builder(c.JavaType, c.name.pascalName).build() }
                val whereCodeSegments = it.map { c ->
                    CodeBlock.of(
                        "where(${className}.${c.name.pascalName}.eq(\$T.arg(${c.name.pascalName})))",
                        Expression::class.java
                    )
                }
                MethodSpec.methodBuilder("deleteBy${it.joinToString("And") { c -> c.name.pascalName }}")
                    .addModifiers(Modifier.PUBLIC)
                    .returns(ClassName.BOOLEAN)
                    .addParameters(parameters)
                    .addCode("return this.delete().")
                    .addCode(CodeBlock.join(whereCodeSegments, "."))
                    .addCode(".execute() > 0;")
                    .build()
            }
        //当存在主键/唯一列的时候,添加Save方法,用于将刚刚保存的实体返回
        val saveMethods = if (uniqueColumns.isNotEmpty()) {
            //save方法(带选项)
            val saveWithOptionsMethod = MethodSpec.methodBuilder("save")
                .addAnnotation(Nullable::class.java)
                .addModifiers(Modifier.PUBLIC)
                .returns(entityTypeName)
                .addParameter(entityTypeName, "entity")
                .addParameter(ClassName.INT, "options")
            //判断是否有自增列
            val generatedColumn = tableInfo.columns.find { it.isAutoIncrement }
            if (generatedColumn != null) {
                //如果有自增列,则获取它的返回值
                saveWithOptionsMethod.addCode(
                    "\$T generatedColumnValue = this.insert(entity, options);\n",
                    generatedColumn.JavaType
                )
                //如果获取到了值,则给赋值到实体中,以便下面的代码中用来做唯一查找
                saveWithOptionsMethod.addCode("if(generatedColumnValue != null) entity.set${generatedColumn.name.pascalName}(generatedColumnValue);\n")
            } else {
                //否则直接插入即可
                saveWithOptionsMethod.addCode("this.insert(entity, options);\n");
            }
            //如果主键没有/或者没有获取到生成列的值,则使用唯一键查询
            uniqueColumns.forEach {
                val fieldGetters = it.map { c -> "entity.get${c.name.pascalName}()" }
                val condition = fieldGetters.joinToString(" && ") { getter -> "$getter != null" }
                val findMethodName = "findBy${it.joinToString("And") { c -> c.name.pascalName }}"
                val parameters = fieldGetters.joinToString(", ")
                saveWithOptionsMethod.addCode("if($condition) return this.$findMethodName($parameters);\n")
            }
            //最后返回空
            saveWithOptionsMethod.addCode("return null;")
            //save方法
            val saveMethod = MethodSpec.methodBuilder("save")
                .addAnnotation(Nullable::class.java)
                .addModifiers(Modifier.PUBLIC)
                .returns(entityTypeName)
                .addParameter(entityTypeName, "entity")
                .addCode("return this.save(entity, \$T.NULL_IS_NONE);", InsertOption::class.java)
            //saveOrUpdate方法
            val saveOrUpdateMethod = MethodSpec.methodBuilder("saveOrUpdate")
                .addAnnotation(Nullable::class.java)
                .addModifiers(Modifier.PUBLIC)
                .returns(entityTypeName)
                .addParameter(entityTypeName, "entity")
                .addCode(
                    "return this.save(entity, \$T.NULL_IS_NONE | \$T.INSERT_OR_UPDATE);",
                    InsertOption::class.java,
                    InsertOption::class.java
                )
            listOf(saveWithOptionsMethod.build(), saveMethod.build(), saveOrUpdateMethod.build())
        } else {
            listOf()
        }
        return findMethods + deleteMethods + saveMethods
    }
}
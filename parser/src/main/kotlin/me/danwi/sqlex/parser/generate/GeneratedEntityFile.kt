package me.danwi.sqlex.parser.generate

import com.squareup.javapoet.*
import me.danwi.sqlex.parser.Repository
import me.danwi.sqlex.parser.util.pascalName
import javax.lang.model.element.Modifier

class GeneratedEntityFile(
    rootPackage: String,
    private val tableName: String,
    private val repository: Repository,
) : GeneratedJavaFile(rootPackage, tableName.pascalName) {
    override fun generate(): TypeSpec {
        val tableInfo = repository.getTableInfo(tableName)
        val entityType = tableInfo.columns.toEntityClass(tableName.pascalName)
        //添加builder
        val builders = mutableListOf<TypeSpec>()
        //获取必要字段,必要字段指的是 不能null,且不能自动生成(不自增 && 没有默认值)
        val necessaryColumns = tableInfo.columns.filter { it.notNull && (!it.isAutoIncrement && !it.hasDefaultValue) }
        //获取非必要字段
        val unnecessaryColumns = tableInfo.columns.filter { !necessaryColumns.contains(it) }
        builders.add(TypeSpec
            .classBuilder("OptionBuilder")
            .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
            .addFields(tableInfo.columns.map {
                FieldSpec.builder(it.JavaType, it.name.pascalName, Modifier.PRIVATE).build()
            })
            .addMethod(
                MethodSpec.constructorBuilder()
                    .addParameters(necessaryColumns.map {
                        ParameterSpec.builder(it.JavaType, it.name.pascalName).build()
                    })
                    .apply {
                        necessaryColumns.forEach { this.addCode("this.${it.name.pascalName} = ${it.name.pascalName};\n") }
                    }
                    .build()
            )
            .addMethods(
                unnecessaryColumns.map {
                    MethodSpec.methodBuilder("set${it.name.pascalName}")
                        .addModifiers(Modifier.PUBLIC)
                        .addParameter(it.JavaType, "value")
                        .addCode("this.${it.name.pascalName} = value;\n")
                        .addCode("return this;\n")
                        .returns(ClassName.get(packageName, className, "OptionBuilder"))
                        .build()
                }
            )
            .addMethod(
                MethodSpec.methodBuilder("build")
                    .addModifiers(Modifier.PUBLIC)
                    .addCode("return new $className(${tableInfo.columns.joinToString { "this.${it.name.pascalName}" }});\n")
                    .returns(ClassName.get(packageName, className))
                    .build()
            )
            .build()
        )
        val builderFields = necessaryColumns.indices.map { necessaryColumns.slice(0..it) }.reversed()
        for (fields in builderFields) {
            //构造类
            val currentField = fields.last()
            val builderType = TypeSpec
                .classBuilder("${currentField.name.pascalName}Initial")
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
            //字段
            builderType.addFields(fields.map {
                FieldSpec.builder(it.JavaType, it.name.pascalName, Modifier.PRIVATE).build()
            })
            //构造函数
            val prevFields = fields.slice(0 until fields.size - 1)
            builderType.addMethod(
                MethodSpec.constructorBuilder()
                    .addParameters(prevFields.map { ParameterSpec.builder(it.JavaType, it.name.pascalName).build() })
                    .apply {
                        prevFields.forEach { this.addCode("this.${it.name.pascalName} = ${it.name.pascalName};\n") }
                    }
                    .build()
            )
            //设值函数
            val nextBuilder = builders.last()
            builderType.addMethod(
                MethodSpec.methodBuilder("set${currentField.name.pascalName}")
                    .addModifiers(Modifier.PUBLIC)
                    .addParameter(currentField.JavaType, "value")
                    .addCode("this.${currentField.name.pascalName} = value;\n")
                    .addCode("return new ${nextBuilder.name}(${fields.joinToString { "this.${it.name.pascalName}" }});\n")
                    .returns(ClassName.get(packageName, className, nextBuilder.name))
                    .build()
            )
            builders.add(builderType.build())
        }
        //重新builder
        val entityTypeSpec = entityType.toBuilder()
        entityTypeSpec.addTypes(builders)
        //添加builder函数
        val firstBuilder = builders.last()
        entityTypeSpec.addMethod(
            MethodSpec.methodBuilder("builder")
                .addModifiers(Modifier.PUBLIC, Modifier.STATIC)
                .addCode("return new ${firstBuilder.name}();\n")
                .returns(ClassName.get(packageName, className, firstBuilder.name))
                .build()
        )
        return entityTypeSpec.build()
    }
}
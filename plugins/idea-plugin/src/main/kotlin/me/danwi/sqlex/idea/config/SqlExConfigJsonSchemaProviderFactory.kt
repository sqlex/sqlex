package me.danwi.sqlex.idea.config

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VfsUtil
import com.intellij.openapi.vfs.VirtualFile
import com.jetbrains.jsonSchema.extension.JsonSchemaFileProvider
import com.jetbrains.jsonSchema.extension.JsonSchemaProviderFactory
import com.jetbrains.jsonSchema.extension.SchemaType
import me.danwi.sqlex.idea.util.extension.isSqlExConfig
import java.util.*

class SqlExConfigJsonSchemaProviderFactory : JsonSchemaProviderFactory {
    override fun getProviders(project: Project): MutableList<JsonSchemaFileProvider> {
        return Collections.singletonList(object : JsonSchemaFileProvider {
            override fun isAvailable(file: VirtualFile): Boolean {
                return file.isSqlExConfig
            }

            override fun getName(): String {
                return "SqlEx Config"
            }

            override fun getSchemaFile(): VirtualFile? {
                return VfsUtil.findFileByURL(
                    javaClass.getResource("/schemas/sqlex.config.schema.json") ?: return null
                )
            }

            override fun getSchemaType(): SchemaType {
                return SchemaType.embeddedSchema
            }
        })
    }
}
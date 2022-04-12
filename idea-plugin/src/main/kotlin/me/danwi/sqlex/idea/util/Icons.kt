package me.danwi.sqlex.idea.util

import com.intellij.ui.IconManager
import com.intellij.util.IconUtil

val SqlExIcon = IconManager.getInstance().getIcon("icons/sqlex.svg", object {}::class.java)

val SqlExIconX10 = IconUtil.resizeSquared(SqlExIcon, 10)
val SqlExIconX16 = IconUtil.resizeSquared(SqlExIcon, 16)
val SqlExIconX32 = IconUtil.resizeSquared(SqlExIcon, 32)

val SqlExConfigFileIcon = IconUtil.addText(SqlExIconX16, "C")
val SqlExSchemaFileIcon = IconUtil.addText(SqlExIconX16, "S")
val SqlExMethodFileIcon = IconUtil.addText(SqlExIconX16, "M")
val SqlExMethodIcon = SqlExMethodFileIcon